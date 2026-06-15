// SPDX-License-Identifier: Apache-2.0

//! PostgreSQL and SQLite persistence for ADS entities.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use ga4gh_types::{
    AccessDecision, AccessDecisionOutcome, AccessRequest, AccessRequestStatus, AdsEvent,
    AdsEventType, CreateAccessRequestBody, CreateDatasetRequest, CreatePermissionMappingRequest,
    CreatePermissionSourceRequest, CreateProjectRequest, CreateVisaSourceRequest, Dataset, DuoCode,
    DuoEvaluationResult, Grant, GrantSource, PermissionMapping, PermissionSource, ResearchProject,
    Researcher, VisaSource,
};
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

use crate::auth::{hash_api_key, verify_api_key};
use crate::config::{DatabaseConfig, DatabaseDriver};
use crate::error::AdsError;
use crate::events::{
    grant_created, grant_revoked, request_approved, request_created, request_rejected,
};

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[derive(Clone)]
enum DbPool {
    #[cfg(feature = "postgres")]
    Postgres(PgPool),
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
}

/// Database-backed ADS store.
#[derive(Clone)]
pub struct AdsStore {
    pool: DbPool,
    webhook_urls: Arc<Vec<String>>,
}

/// Joined permission mapping row used for institutional grant evaluation.
struct ActivePermissionMapping {
    claim_path: String,
    claim_value: String,
    dataset_id: Uuid,
    grant_lifetime_seconds: Option<u64>,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn dt_from_ts(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

fn map_db_err(err: impl ToString) -> AdsError {
    AdsError::Database(err.to_string())
}

macro_rules! parse_researcher {
    ($row:expr) => {{
        let row = $row;
        let affiliations_json: String = row.try_get("affiliations").map_err(map_db_err)?;
        let affiliations: Vec<ga4gh_types::ResearcherAffiliation> =
            serde_json::from_str(&affiliations_json).map_err(map_db_err)?;
        Researcher {
            id: row.try_get("id").map_err(map_db_err)?,
            display_name: row.try_get("display_name").map_err(map_db_err)?,
            email: row.try_get("email").map_err(map_db_err)?,
            affiliations,
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_dataset {
    ($row:expr) => {{
        let row = $row;
        let duo_json: String = row.try_get("duo_codes").map_err(map_db_err)?;
        let duo_codes: Vec<DuoCode> = serde_json::from_str(&duo_json).map_err(map_db_err)?;
        Dataset {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            name: row.try_get("name").map_err(map_db_err)?,
            description: row.try_get("description").map_err(map_db_err)?,
            duo_codes,
            external_id: row.try_get("external_id").map_err(map_db_err)?,
            auto_approve_enabled: row
                .try_get::<i32, _>("auto_approve_enabled")
                .map_err(map_db_err)?
                != 0,
            auto_approve_threshold: row
                .try_get::<i32, _>("auto_approve_threshold")
                .map_err(map_db_err)? as u8,
            dac_group: row.try_get("dac_group").map_err(map_db_err)?,
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_project {
    ($row:expr) => {{
        let row = $row;
        let duo_json: String = row.try_get("duo_codes").map_err(map_db_err)?;
        let duo_codes: Vec<DuoCode> = serde_json::from_str(&duo_json).map_err(map_db_err)?;
        ResearchProject {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            researcher_id: row.try_get("researcher_id").map_err(map_db_err)?,
            name: row.try_get("name").map_err(map_db_err)?,
            description: row.try_get("description").map_err(map_db_err)?,
            duo_codes,
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_access_request {
    ($row:expr) => {{
        let row = $row;
        let eval_json: Option<String> = row.try_get("duo_evaluation").map_err(map_db_err)?;
        let duo_evaluation = eval_json
            .map(|json| serde_json::from_str::<DuoEvaluationResult>(&json))
            .transpose()
            .map_err(map_db_err)?;
        AccessRequest {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            researcher_id: row.try_get("researcher_id").map_err(map_db_err)?,
            dataset_id: Uuid::parse_str(
                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
            )
            .map_err(map_db_err)?,
            project_id: Uuid::parse_str(
                &row.try_get::<String, _>("project_id").map_err(map_db_err)?,
            )
            .map_err(map_db_err)?,
            status: parse_status(&row.try_get::<String, _>("status").map_err(map_db_err)?)?,
            justification: row.try_get("justification").map_err(map_db_err)?,
            duo_evaluation,
            dac_group: row.try_get("dac_group").map_err(map_db_err)?,
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_grant {
    ($row:expr) => {{
        let row = $row;
        let duo_json: String = row.try_get("duo_codes").map_err(map_db_err)?;
        let duo_codes: Vec<DuoCode> = serde_json::from_str(&duo_json).map_err(map_db_err)?;
        let request_id: Option<String> = row.try_get("request_id").map_err(map_db_err)?;
        Grant {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            researcher_id: row.try_get("researcher_id").map_err(map_db_err)?,
            dataset_id: Uuid::parse_str(
                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
            )
            .map_err(map_db_err)?,
            request_id: request_id
                .map(|id| Uuid::parse_str(&id))
                .transpose()
                .map_err(map_db_err)?,
            source: parse_grant_source(&row.try_get::<String, _>("source").map_err(map_db_err)?)?,
            duo_codes,
            resource_scope: row.try_get("resource_scope").map_err(map_db_err)?,
            expires_at: row
                .try_get::<Option<i64>, _>("expires_at")
                .map_err(map_db_err)?
                .map(dt_from_ts),
            revoked_at: row
                .try_get::<Option<i64>, _>("revoked_at")
                .map_err(map_db_err)?
                .map(dt_from_ts),
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_audit_event {
    ($row:expr) => {{
        let payload_raw: String = $row.try_get("payload").map_err(map_db_err)?;
        let payload: BTreeMap<String, serde_json::Value> =
            serde_json::from_str(&payload_raw).map_err(map_db_err)?;
        AdsEvent {
            id: Uuid::parse_str(&$row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            event_type: parse_event_type(
                &$row
                    .try_get::<String, _>("event_type")
                    .map_err(map_db_err)?,
            )?,
            occurred_at: dt_from_ts($row.try_get("occurred_at").map_err(map_db_err)?),
            payload,
        }
    }};
}

impl AdsStore {
    pub async fn connect(
        database: &DatabaseConfig,
        url: &str,
        webhook_urls: Vec<String>,
    ) -> Result<Self, AdsError> {
        match database.driver {
            #[cfg(feature = "postgres")]
            DatabaseDriver::Postgres => {
                let pool = PgPool::connect(url).await.map_err(map_db_err)?;
                if database.auto_migrate {
                    sqlx::migrate!().run(&pool).await.map_err(map_db_err)?;
                }
                Ok(Self {
                    pool: DbPool::Postgres(pool),
                    webhook_urls: Arc::new(webhook_urls),
                })
            }
            #[cfg(feature = "sqlite")]
            DatabaseDriver::Sqlite => Self::connect_sqlite(url, webhook_urls).await,
            #[cfg(not(feature = "postgres"))]
            DatabaseDriver::Postgres => Err(AdsError::Config(
                "ADS was built without the `postgres` feature".to_string(),
            )),
            #[cfg(not(feature = "sqlite"))]
            DatabaseDriver::Sqlite => Err(AdsError::Config(
                "ADS was built without the `sqlite` feature".to_string(),
            )),
        }
    }

    #[cfg(feature = "sqlite")]
    async fn connect_sqlite(url: &str, webhook_urls: Vec<String>) -> Result<Self, AdsError> {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let options = SqliteConnectOptions::from_str(url)
            .map_err(|err| AdsError::Database(format!("invalid SQLite URL: {err}")))?
            .create_if_missing(true);

        if !options.get_filename().as_os_str().is_empty() {
            if let Some(parent) = options.get_filename().parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|err| {
                        AdsError::Database(format!(
                            "creating SQLite directory `{}`: {err}",
                            parent.display()
                        ))
                    })?;
                }
            }
        }

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(map_db_err)?;
        sqlx::migrate!().run(&pool).await.map_err(map_db_err)?;
        Ok(Self {
            pool: DbPool::Sqlite(pool),
            webhook_urls: Arc::new(webhook_urls),
        })
    }

    pub async fn ensure_bootstrap_api_key(
        &self,
        raw_key: &str,
        name: &str,
    ) -> Result<(), AdsError> {
        if self.count_active_api_keys().await? > 0 {
            return Ok(());
        }
        self.insert_api_key(name, raw_key).await
    }

    async fn count_active_api_keys(&self) -> Result<i64, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row =
                    sqlx::query("SELECT COUNT(*) AS count FROM api_keys WHERE revoked_at IS NULL")
                        .fetch_one(pool)
                        .await
                        .map_err(map_db_err)?;
                Ok(row.get::<i64, _>("count"))
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row =
                    sqlx::query("SELECT COUNT(*) AS count FROM api_keys WHERE revoked_at IS NULL")
                        .fetch_one(pool)
                        .await
                        .map_err(map_db_err)?;
                Ok(row.get::<i64, _>("count"))
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    async fn insert_api_key(&self, name: &str, raw_key: &str) -> Result<(), AdsError> {
        let id = Uuid::new_v4().to_string();
        let key_hash = hash_api_key(raw_key);
        let now = unix_now();
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO api_keys (id, name, key_hash, created_at) VALUES ($1, $2, $3, $4)",
                )
                .bind(&id)
                .bind(name)
                .bind(&key_hash)
                .bind(now)
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO api_keys (id, name, key_hash, created_at) VALUES ($1, $2, $3, $4)",
                )
                .bind(&id)
                .bind(name)
                .bind(&key_hash)
                .bind(now)
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    pub async fn verify_api_key(&self, raw_key: &str) -> Result<Option<String>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows =
                    sqlx::query("SELECT name, key_hash FROM api_keys WHERE revoked_at IS NULL")
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?;
                for row in rows {
                    let name: String = row.get("name");
                    let hash: String = row.get("key_hash");
                    if verify_api_key(raw_key, &hash) {
                        return Ok(Some(name));
                    }
                }
                Ok(None)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows =
                    sqlx::query("SELECT name, key_hash FROM api_keys WHERE revoked_at IS NULL")
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?;
                for row in rows {
                    let name: String = row.get("name");
                    let hash: String = row.get("key_hash");
                    if verify_api_key(raw_key, &hash) {
                        return Ok(Some(name));
                    }
                }
                Ok(None)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn upsert_researcher(&self, researcher: &Researcher) -> Result<(), AdsError> {
        let affiliations = serde_json::to_string(&researcher.affiliations).map_err(map_db_err)?;
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO researchers (id, display_name, email, affiliations, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT(id) DO UPDATE SET
                       display_name = EXCLUDED.display_name,
                       email = EXCLUDED.email,
                       affiliations = EXCLUDED.affiliations,
                       updated_at = EXCLUDED.updated_at",
                )
                .bind(&researcher.id)
                .bind(&researcher.display_name)
                .bind(&researcher.email)
                .bind(&affiliations)
                .bind(researcher.created_at.timestamp())
                .bind(researcher.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO researchers (id, display_name, email, affiliations, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT(id) DO UPDATE SET
                       display_name = excluded.display_name,
                       email = excluded.email,
                       affiliations = excluded.affiliations,
                       updated_at = excluded.updated_at",
                )
                .bind(&researcher.id)
                .bind(&researcher.display_name)
                .bind(&researcher.email)
                .bind(&affiliations)
                .bind(researcher.created_at.timestamp())
                .bind(researcher.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    pub async fn get_researcher(&self, id: &str) -> Result<Researcher, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, display_name, email, affiliations, created_at, updated_at
                     FROM researchers WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Researcher, AdsError> { Ok(parse_researcher!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, display_name, email, affiliations, created_at, updated_at
                     FROM researchers WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Researcher, AdsError> { Ok(parse_researcher!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn create_dataset(&self, req: &CreateDatasetRequest) -> Result<Dataset, AdsError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let duo_json = serde_json::to_string(&req.duo_codes).map_err(map_db_err)?;
        let dataset = Dataset {
            id,
            name: req.name.clone(),
            description: req.description.clone(),
            duo_codes: req.duo_codes.clone(),
            external_id: req.external_id.clone(),
            auto_approve_enabled: req.auto_approve_enabled,
            auto_approve_threshold: req.auto_approve_threshold,
            dac_group: req.dac_group.clone(),
            created_at: now,
            updated_at: now,
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO datasets (id, name, description, duo_codes, external_id,
                     auto_approve_enabled, auto_approve_threshold, dac_group, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(dataset.id.to_string())
                .bind(&dataset.name)
                .bind(&dataset.description)
                .bind(&duo_json)
                .bind(&dataset.external_id)
                .bind(i64::from(dataset.auto_approve_enabled))
                .bind(i64::from(dataset.auto_approve_threshold))
                .bind(&dataset.dac_group)
                .bind(dataset.created_at.timestamp())
                .bind(dataset.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO datasets (id, name, description, duo_codes, external_id,
                     auto_approve_enabled, auto_approve_threshold, dac_group, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(dataset.id.to_string())
                .bind(&dataset.name)
                .bind(&dataset.description)
                .bind(&duo_json)
                .bind(&dataset.external_id)
                .bind(i64::from(dataset.auto_approve_enabled))
                .bind(i64::from(dataset.auto_approve_threshold))
                .bind(&dataset.dac_group)
                .bind(dataset.created_at.timestamp())
                .bind(dataset.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(dataset)
    }

    pub async fn list_datasets(
        &self,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<Dataset>, AdsError> {
        let select = "SELECT id, name, description, duo_codes, external_id,
                            auto_approve_enabled, auto_approve_threshold, dac_group, created_at, updated_at
                     FROM datasets";
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders: Vec<String> =
                        (1..=groups.len()).map(|i| format!("${i}")).collect();
                    let sql = format!(
                        "{select} WHERE dac_group IN ({}) ORDER BY created_at DESC",
                        placeholders.join(", ")
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at DESC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
                rows.into_iter()
                    .map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders = std::iter::repeat_n("?", groups.len())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let sql = format!(
                        "{select} WHERE dac_group IN ({placeholders}) ORDER BY created_at DESC"
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at DESC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
                rows.into_iter()
                    .map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn get_dataset(&self, id: Uuid) -> Result<Dataset, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, description, duo_codes, external_id,
                            auto_approve_enabled, auto_approve_threshold, dac_group, created_at, updated_at
                     FROM datasets WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, description, duo_codes, external_id,
                            auto_approve_enabled, auto_approve_threshold, dac_group, created_at, updated_at
                     FROM datasets WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn create_project(
        &self,
        req: &CreateProjectRequest,
    ) -> Result<ResearchProject, AdsError> {
        self.ensure_researcher_exists(&req.researcher_id).await?;
        let id = Uuid::new_v4();
        let now = Utc::now();
        let duo_json = serde_json::to_string(&req.duo_codes).map_err(map_db_err)?;
        let project = ResearchProject {
            id,
            researcher_id: req.researcher_id.clone(),
            name: req.name.clone(),
            description: req.description.clone(),
            duo_codes: req.duo_codes.clone(),
            created_at: now,
            updated_at: now,
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO research_projects (id, researcher_id, name, description, duo_codes, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(project.id.to_string())
                .bind(&project.researcher_id)
                .bind(&project.name)
                .bind(&project.description)
                .bind(&duo_json)
                .bind(project.created_at.timestamp())
                .bind(project.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO research_projects (id, researcher_id, name, description, duo_codes, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(project.id.to_string())
                .bind(&project.researcher_id)
                .bind(&project.name)
                .bind(&project.description)
                .bind(&duo_json)
                .bind(project.created_at.timestamp())
                .bind(project.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(project)
    }

    pub async fn list_projects(&self) -> Result<Vec<ResearchProject>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn get_project(&self, id: Uuid) -> Result<ResearchProject, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    #[instrument(skip(self, body, evaluation))]
    pub async fn create_access_request(
        &self,
        body: &CreateAccessRequestBody,
        evaluation: Option<DuoEvaluationResult>,
    ) -> Result<AccessRequest, AdsError> {
        self.ensure_researcher_exists(&body.researcher_id).await?;
        let dataset = self.get_dataset(body.dataset_id).await?;
        let project = self.get_project(body.project_id).await?;
        if project.researcher_id != body.researcher_id {
            return Err(AdsError::BadRequest(
                "project does not belong to researcher".to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        let eval_json = evaluation
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(map_db_err)?;

        let mut status = AccessRequestStatus::Pending;
        if dataset.auto_approve_enabled && evaluation.as_ref().is_some_and(|e| e.auto_approvable) {
            status = AccessRequestStatus::Approved;
        }

        let request = AccessRequest {
            id,
            researcher_id: body.researcher_id.clone(),
            dataset_id: body.dataset_id,
            project_id: body.project_id,
            status,
            justification: body.justification.clone(),
            duo_evaluation: evaluation,
            dac_group: dataset.dac_group.clone(),
            created_at: now,
            updated_at: now,
        };

        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO access_requests (id, researcher_id, dataset_id, project_id, status,
                     justification, duo_evaluation, dac_group, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(request.id.to_string())
                .bind(&request.researcher_id)
                .bind(request.dataset_id.to_string())
                .bind(request.project_id.to_string())
                .bind(status_str(request.status))
                .bind(&request.justification)
                .bind(eval_json)
                .bind(&request.dac_group)
                .bind(request.created_at.timestamp())
                .bind(request.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO access_requests (id, researcher_id, dataset_id, project_id, status,
                     justification, duo_evaluation, dac_group, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(request.id.to_string())
                .bind(&request.researcher_id)
                .bind(request.dataset_id.to_string())
                .bind(request.project_id.to_string())
                .bind(status_str(request.status))
                .bind(&request.justification)
                .bind(eval_json)
                .bind(&request.dac_group)
                .bind(request.created_at.timestamp())
                .bind(request.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }

        request_created(
            self,
            request.id,
            &request.researcher_id,
            request.dataset_id,
            request.dac_group.as_deref(),
        )
        .await?;

        if request.status == AccessRequestStatus::Approved {
            self.record_decision(
                request.id,
                AccessDecisionOutcome::Approved,
                "system:duo-auto",
                Some("automatic DUO approval".to_string()),
            )
            .await?;
            self.create_grant_from_request(&request, GrantSource::DuoAutoApproval)
                .await?;
            request_approved(self, request.id, request.dac_group.as_deref()).await?;
        }

        Ok(request)
    }

    pub async fn get_access_request(&self, id: Uuid) -> Result<AccessRequest, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, project_id, status, justification,
                            duo_evaluation, dac_group, created_at, updated_at
                     FROM access_requests WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<AccessRequest, AdsError> {
                    Ok(parse_access_request!(&row))
                })
                .transpose()?
                .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, project_id, status, justification,
                            duo_evaluation, dac_group, created_at, updated_at
                     FROM access_requests WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<AccessRequest, AdsError> {
                    Ok(parse_access_request!(&row))
                })
                .transpose()?
                .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn list_dac_requests(
        &self,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<AccessRequest>, AdsError> {
        let select = "SELECT id, researcher_id, dataset_id, project_id, status, justification,
                            duo_evaluation, dac_group, created_at, updated_at
                     FROM access_requests
                     WHERE status IN ('pending', 'escalated')";
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders: Vec<String> =
                        (1..=groups.len()).map(|i| format!("${i}")).collect();
                    let sql = format!(
                        "{select} AND dac_group IN ({}) ORDER BY created_at ASC",
                        placeholders.join(", ")
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at ASC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
                rows.into_iter()
                    .map(|row| -> Result<AccessRequest, AdsError> {
                        Ok(parse_access_request!(&row))
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders = std::iter::repeat_n("?", groups.len())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let sql = format!(
                        "{select} AND dac_group IN ({placeholders}) ORDER BY created_at ASC"
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at ASC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
                rows.into_iter()
                    .map(|row| -> Result<AccessRequest, AdsError> {
                        Ok(parse_access_request!(&row))
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn dac_approve(
        &self,
        id: Uuid,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessRequest, AdsError> {
        let mut request = self.get_access_request(id).await?;
        if !matches!(
            request.status,
            AccessRequestStatus::Pending | AccessRequestStatus::Escalated
        ) {
            return Err(AdsError::Conflict(format!(
                "request is {:?}, not reviewable",
                request.status
            )));
        }
        self.record_decision(id, AccessDecisionOutcome::Approved, actor, reason)
            .await?;
        request.status = AccessRequestStatus::Approved;
        request.updated_at = Utc::now();
        self.update_request_status(&request).await?;
        self.create_grant_from_request(&request, GrantSource::DacApproval)
            .await?;
        request_approved(self, id, request.dac_group.as_deref()).await?;
        Ok(request)
    }

    pub async fn dac_reject(
        &self,
        id: Uuid,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessRequest, AdsError> {
        let mut request = self.get_access_request(id).await?;
        if !matches!(
            request.status,
            AccessRequestStatus::Pending | AccessRequestStatus::Escalated
        ) {
            return Err(AdsError::Conflict(format!(
                "request is {:?}, not reviewable",
                request.status
            )));
        }
        self.record_decision(id, AccessDecisionOutcome::Rejected, actor, reason)
            .await?;
        request.status = AccessRequestStatus::Rejected;
        request.updated_at = Utc::now();
        self.update_request_status(&request).await?;
        request_rejected(self, id, request.dac_group.as_deref()).await?;
        Ok(request)
    }

    pub async fn dac_escalate(
        &self,
        id: Uuid,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessRequest, AdsError> {
        let mut request = self.get_access_request(id).await?;
        if request.status != AccessRequestStatus::Pending {
            return Err(AdsError::Conflict(format!(
                "request is {:?}, cannot escalate",
                request.status
            )));
        }
        self.record_decision(id, AccessDecisionOutcome::Escalated, actor, reason)
            .await?;
        request.status = AccessRequestStatus::Escalated;
        request.updated_at = Utc::now();
        self.update_request_status(&request).await?;
        Ok(request)
    }

    async fn record_decision(
        &self,
        request_id: Uuid,
        outcome: AccessDecisionOutcome,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessDecision, AdsError> {
        let decision = AccessDecision {
            id: Uuid::new_v4(),
            request_id,
            outcome,
            actor: actor.to_string(),
            reason,
            decided_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO access_decisions (id, request_id, outcome, actor, reason, decided_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(decision.id.to_string())
                .bind(decision.request_id.to_string())
                .bind(outcome_str(decision.outcome))
                .bind(&decision.actor)
                .bind(&decision.reason)
                .bind(decision.decided_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO access_decisions (id, request_id, outcome, actor, reason, decided_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(decision.id.to_string())
                .bind(decision.request_id.to_string())
                .bind(outcome_str(decision.outcome))
                .bind(&decision.actor)
                .bind(&decision.reason)
                .bind(decision.decided_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(decision)
    }

    async fn update_request_status(&self, request: &AccessRequest) -> Result<(), AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE access_requests SET status = $1, updated_at = $2 WHERE id = $3",
                )
                .bind(status_str(request.status))
                .bind(request.updated_at.timestamp())
                .bind(request.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE access_requests SET status = $1, updated_at = $2 WHERE id = $3",
                )
                .bind(status_str(request.status))
                .bind(request.updated_at.timestamp())
                .bind(request.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    async fn create_grant_from_request(
        &self,
        request: &AccessRequest,
        source: GrantSource,
    ) -> Result<Grant, AdsError> {
        let dataset = self.get_dataset(request.dataset_id).await?;
        let grant = Grant {
            id: Uuid::new_v4(),
            researcher_id: request.researcher_id.clone(),
            dataset_id: request.dataset_id,
            request_id: Some(request.id),
            source,
            duo_codes: dataset.duo_codes.clone(),
            resource_scope: dataset.external_id.clone(),
            expires_at: None,
            revoked_at: None,
            created_at: Utc::now(),
        };
        self.insert_grant(&grant).await?;
        grant_created(
            self,
            grant.id,
            &grant.researcher_id,
            grant.dataset_id,
            dataset.dac_group.as_deref(),
        )
        .await?;
        Ok(grant)
    }

    pub async fn insert_grant(&self, grant: &Grant) -> Result<(), AdsError> {
        let duo_json = serde_json::to_string(&grant.duo_codes).map_err(map_db_err)?;
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO grants (id, researcher_id, dataset_id, request_id, source, duo_codes,
                     resource_scope, expires_at, revoked_at, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(grant.id.to_string())
                .bind(&grant.researcher_id)
                .bind(grant.dataset_id.to_string())
                .bind(grant.request_id.map(|id| id.to_string()))
                .bind(grant_source_str(&grant.source))
                .bind(&duo_json)
                .bind(&grant.resource_scope)
                .bind(grant.expires_at.map(|dt| dt.timestamp()))
                .bind(grant.revoked_at.map(|dt| dt.timestamp()))
                .bind(grant.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO grants (id, researcher_id, dataset_id, request_id, source, duo_codes,
                     resource_scope, expires_at, revoked_at, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(grant.id.to_string())
                .bind(&grant.researcher_id)
                .bind(grant.dataset_id.to_string())
                .bind(grant.request_id.map(|id| id.to_string()))
                .bind(grant_source_str(&grant.source))
                .bind(&duo_json)
                .bind(&grant.resource_scope)
                .bind(grant.expires_at.map(|dt| dt.timestamp()))
                .bind(grant.revoked_at.map(|dt| dt.timestamp()))
                .bind(grant.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    pub async fn list_grants(
        &self,
        researcher_id: Option<&str>,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<Grant>, AdsError> {
        let grant_cols = "g.id, g.researcher_id, g.dataset_id, g.request_id, g.source, g.duo_codes,
                                g.resource_scope, g.expires_at, g.revoked_at, g.created_at";
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = match (researcher_id, dac_groups.filter(|g| !g.is_empty())) {
                    (Some(sub), Some(groups)) => {
                        let placeholders: Vec<String> =
                            (2..=groups.len() + 1).map(|i| format!("${i}")).collect();
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.researcher_id = $1 AND g.revoked_at IS NULL
                             AND d.dac_group IN ({})",
                            placeholders.join(", ")
                        );
                        let mut query = sqlx::query(&sql).bind(sub);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (Some(sub), None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g
                             WHERE g.researcher_id = $1 AND g.revoked_at IS NULL"
                    ))
                    .bind(sub)
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                    (None, Some(groups)) => {
                        let placeholders: Vec<String> =
                            (1..=groups.len()).map(|i| format!("${i}")).collect();
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.revoked_at IS NULL AND d.dac_group IN ({})",
                            placeholders.join(", ")
                        );
                        let mut query = sqlx::query(&sql);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (None, None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g WHERE g.revoked_at IS NULL"
                    ))
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                };
                rows.into_iter()
                    .map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = match (researcher_id, dac_groups.filter(|g| !g.is_empty())) {
                    (Some(sub), Some(groups)) => {
                        let placeholders = std::iter::repeat_n("?", groups.len())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.researcher_id = ? AND g.revoked_at IS NULL
                             AND d.dac_group IN ({placeholders})"
                        );
                        let mut query = sqlx::query(&sql).bind(sub);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (Some(sub), None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g
                             WHERE g.researcher_id = ? AND g.revoked_at IS NULL"
                    ))
                    .bind(sub)
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                    (None, Some(groups)) => {
                        let placeholders = std::iter::repeat_n("?", groups.len())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.revoked_at IS NULL AND d.dac_group IN ({placeholders})"
                        );
                        let mut query = sqlx::query(&sql);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (None, None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g WHERE g.revoked_at IS NULL"
                    ))
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                };
                rows.into_iter()
                    .map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn get_grant(&self, id: Uuid) -> Result<Grant, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, request_id, source, duo_codes,
                            resource_scope, expires_at, revoked_at, created_at
                     FROM grants WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, request_id, source, duo_codes,
                            resource_scope, expires_at, revoked_at, created_at
                     FROM grants WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn revoke_grant(&self, id: Uuid) -> Result<Grant, AdsError> {
        let mut grant = self.get_grant(id).await?;
        if grant.revoked_at.is_some() {
            return Err(AdsError::Conflict("grant already revoked".to_string()));
        }
        grant.revoked_at = Some(Utc::now());
        let revoked_at = grant.revoked_at.unwrap().timestamp();
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query("UPDATE grants SET revoked_at = $1 WHERE id = $2")
                    .bind(revoked_at)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query("UPDATE grants SET revoked_at = $1 WHERE id = $2")
                    .bind(revoked_at)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
            }
        }
        grant_revoked(self, id).await?;
        Ok(grant)
    }

    pub async fn active_grants_for_resource(
        &self,
        researcher_id: &str,
        dataset_id: Option<Uuid>,
        resource: &str,
    ) -> Result<Vec<Grant>, AdsError> {
        let grants = self.list_grants(Some(researcher_id), None).await?;
        Ok(grants
            .into_iter()
            .filter(|g| {
                if g.revoked_at.is_some() {
                    return false;
                }
                if let Some(exp) = g.expires_at {
                    if exp <= Utc::now() {
                        return false;
                    }
                }
                if let Some(ds) = dataset_id {
                    if g.dataset_id != ds {
                        return false;
                    }
                }
                if let Some(scope) = &g.resource_scope {
                    return scope == resource || resource.contains(scope.as_str());
                }
                true
            })
            .collect())
    }

    pub async fn create_visa_source(
        &self,
        req: &CreateVisaSourceRequest,
    ) -> Result<VisaSource, AdsError> {
        let source = VisaSource {
            id: Uuid::new_v4(),
            name: req.name.clone(),
            issuer_url: req.issuer_url.clone(),
            visa_type: req.visa_type.clone(),
            enabled: req.enabled,
            created_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO visa_sources (id, name, issuer_url, visa_type, enabled, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.issuer_url)
                .bind(source.visa_type.to_string())
                .bind(i64::from(source.enabled))
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO visa_sources (id, name, issuer_url, visa_type, enabled, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.issuer_url)
                .bind(source.visa_type.to_string())
                .bind(i64::from(source.enabled))
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(source)
    }

    pub async fn create_permission_source(
        &self,
        req: &CreatePermissionSourceRequest,
    ) -> Result<PermissionSource, AdsError> {
        let source = PermissionSource {
            id: Uuid::new_v4(),
            name: req.name.clone(),
            oidc_issuer: req.oidc_issuer.clone(),
            claim_path: req.claim_path.clone(),
            created_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO permission_sources (id, name, oidc_issuer, claim_path, created_at)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.oidc_issuer)
                .bind(&source.claim_path)
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO permission_sources (id, name, oidc_issuer, claim_path, created_at)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.oidc_issuer)
                .bind(&source.claim_path)
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(source)
    }

    pub async fn create_permission_mapping(
        &self,
        req: &CreatePermissionMappingRequest,
    ) -> Result<PermissionMapping, AdsError> {
        let _ = self.get_dataset(req.dataset_id).await?;
        let mapping = PermissionMapping {
            id: Uuid::new_v4(),
            source_id: req.source_id,
            claim_value: req.claim_value.clone(),
            dataset_id: req.dataset_id,
            grant_lifetime_seconds: req.grant_lifetime_seconds,
            created_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO permission_mappings (id, source_id, claim_value, dataset_id,
                     grant_lifetime_seconds, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(mapping.id.to_string())
                .bind(mapping.source_id.to_string())
                .bind(&mapping.claim_value)
                .bind(mapping.dataset_id.to_string())
                .bind(mapping.grant_lifetime_seconds.map(|v| v as i64))
                .bind(mapping.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO permission_mappings (id, source_id, claim_value, dataset_id,
                     grant_lifetime_seconds, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(mapping.id.to_string())
                .bind(mapping.source_id.to_string())
                .bind(&mapping.claim_value)
                .bind(mapping.dataset_id.to_string())
                .bind(mapping.grant_lifetime_seconds.map(|v| v as i64))
                .bind(mapping.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(mapping)
    }

    pub async fn list_permission_sources(&self) -> Result<Vec<PermissionSource>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, name, oidc_issuer, claim_path, created_at
                     FROM permission_sources ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionSource, AdsError> {
                        Ok(PermissionSource {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            name: row.try_get("name").map_err(map_db_err)?,
                            oidc_issuer: row.try_get("oidc_issuer").map_err(map_db_err)?,
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission source timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, name, oidc_issuer, claim_path, created_at
                     FROM permission_sources ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionSource, AdsError> {
                        Ok(PermissionSource {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            name: row.try_get("name").map_err(map_db_err)?,
                            oidc_issuer: row.try_get("oidc_issuer").map_err(map_db_err)?,
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission source timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn list_permission_mappings(&self) -> Result<Vec<PermissionMapping>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, source_id, claim_value, dataset_id, grant_lifetime_seconds, created_at
                     FROM permission_mappings ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionMapping, AdsError> {
                        Ok(PermissionMapping {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            source_id: Uuid::parse_str(
                                &row.try_get::<String, _>("source_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission mapping timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, source_id, claim_value, dataset_id, grant_lifetime_seconds, created_at
                     FROM permission_mappings ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionMapping, AdsError> {
                        Ok(PermissionMapping {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            source_id: Uuid::parse_str(
                                &row.try_get::<String, _>("source_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission mapping timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn delete_permission_mapping(&self, id: Uuid) -> Result<(), AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM permission_mappings WHERE id = $1")
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
                if result.rows_affected() == 0 {
                    return Err(AdsError::NotFound);
                }
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let result = sqlx::query("DELETE FROM permission_mappings WHERE id = $1")
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
                if result.rows_affected() == 0 {
                    return Err(AdsError::NotFound);
                }
            }
            #[allow(unreachable_patterns)]
            _ => return Err(AdsError::Config("no database driver enabled".to_string())),
        }
        Ok(())
    }

    pub async fn apply_institutional_mappings(
        &self,
        researcher_id: &str,
        claims: &BTreeMap<String, serde_json::Value>,
    ) -> Result<Vec<Grant>, AdsError> {
        use crate::permissions::{claim_values, grant_from_mapping};

        let mappings = self.list_active_permission_mappings().await?;
        let mut created = Vec::new();
        for mapping in mappings {
            let values = claim_values(claims, &mapping.claim_path);
            if !values.iter().any(|value| value == &mapping.claim_value) {
                continue;
            }
            if self
                .has_active_grant(researcher_id, mapping.dataset_id)
                .await?
            {
                continue;
            }
            let dataset = self.get_dataset(mapping.dataset_id).await?;
            let grant = grant_from_mapping(
                researcher_id,
                mapping.dataset_id,
                dataset.duo_codes.clone(),
                dataset.external_id.clone(),
                mapping.grant_lifetime_seconds,
            );
            self.insert_grant(&grant).await?;
            grant_created(
                self,
                grant.id,
                researcher_id,
                grant.dataset_id,
                dataset.dac_group.as_deref(),
            )
            .await?;
            created.push(grant);
        }
        Ok(created)
    }

    async fn list_active_permission_mappings(
        &self,
    ) -> Result<Vec<ActivePermissionMapping>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT ps.claim_path, pm.claim_value, pm.dataset_id, pm.grant_lifetime_seconds
                     FROM permission_mappings pm
                     JOIN permission_sources ps ON ps.id = pm.source_id",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ActivePermissionMapping, AdsError> {
                        Ok(ActivePermissionMapping {
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT ps.claim_path, pm.claim_value, pm.dataset_id, pm.grant_lifetime_seconds
                     FROM permission_mappings pm
                     JOIN permission_sources ps ON ps.id = pm.source_id",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ActivePermissionMapping, AdsError> {
                        Ok(ActivePermissionMapping {
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                        })
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    async fn has_active_grant(
        &self,
        researcher_id: &str,
        dataset_id: Uuid,
    ) -> Result<bool, AdsError> {
        let grants = self.list_grants(Some(researcher_id), None).await?;
        Ok(grants.iter().any(|grant| grant.dataset_id == dataset_id))
    }

    pub fn webhook_urls(&self) -> &[String] {
        &self.webhook_urls
    }

    pub async fn insert_event(&self, event: &AdsEvent) -> Result<(), AdsError> {
        let payload = serde_json::to_string(&event.payload).map_err(map_db_err)?;
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO audit_events (id, event_type, payload, occurred_at)
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(event.id.to_string())
                .bind(event_type_str(&event.event_type))
                .bind(payload)
                .bind(event.occurred_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO audit_events (id, event_type, payload, occurred_at)
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(event.id.to_string())
                .bind(event_type_str(&event.event_type))
                .bind(payload)
                .bind(event.occurred_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    pub async fn list_audit_events(
        &self,
        limit: u32,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<AdsEvent>, AdsError> {
        let mut events: Vec<AdsEvent> = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, event_type, payload, occurred_at
                     FROM audit_events ORDER BY occurred_at DESC LIMIT $1",
                )
                .bind(limit as i64)
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<AdsEvent, AdsError> { Ok(parse_audit_event!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, event_type, payload, occurred_at
                     FROM audit_events ORDER BY occurred_at DESC LIMIT $1",
                )
                .bind(limit as i64)
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<AdsEvent, AdsError> { Ok(parse_audit_event!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }?;
        if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
            events.retain(|event| {
                event
                    .payload
                    .get("dac_group")
                    .and_then(|v| v.as_str())
                    .is_some_and(|g| groups.iter().any(|allowed| allowed == g))
            });
        }
        Ok(events)
    }

    async fn ensure_researcher_exists(&self, id: &str) -> Result<(), AdsError> {
        let now = Utc::now();
        let researcher = Researcher {
            id: id.to_string(),
            display_name: None,
            email: None,
            affiliations: vec![],
            created_at: now,
            updated_at: now,
        };
        self.upsert_researcher(&researcher).await
    }
}

fn status_str(status: AccessRequestStatus) -> &'static str {
    match status {
        AccessRequestStatus::Pending => "pending",
        AccessRequestStatus::Approved => "approved",
        AccessRequestStatus::Rejected => "rejected",
        AccessRequestStatus::Escalated => "escalated",
    }
}

fn parse_status(raw: &str) -> Result<AccessRequestStatus, AdsError> {
    match raw {
        "pending" => Ok(AccessRequestStatus::Pending),
        "approved" => Ok(AccessRequestStatus::Approved),
        "rejected" => Ok(AccessRequestStatus::Rejected),
        "escalated" => Ok(AccessRequestStatus::Escalated),
        other => Err(AdsError::Internal(format!("unknown status {other}"))),
    }
}

fn outcome_str(outcome: AccessDecisionOutcome) -> &'static str {
    match outcome {
        AccessDecisionOutcome::Approved => "approved",
        AccessDecisionOutcome::Rejected => "rejected",
        AccessDecisionOutcome::Escalated => "escalated",
    }
}

fn grant_source_str(source: &GrantSource) -> &'static str {
    match source {
        GrantSource::DacApproval => "dac_approval",
        GrantSource::DuoAutoApproval => "duo_auto_approval",
        GrantSource::InstitutionalMapping => "institutional_mapping",
    }
}

fn parse_grant_source(raw: &str) -> Result<GrantSource, AdsError> {
    match raw {
        "dac_approval" => Ok(GrantSource::DacApproval),
        "duo_auto_approval" => Ok(GrantSource::DuoAutoApproval),
        "institutional_mapping" => Ok(GrantSource::InstitutionalMapping),
        other => Err(AdsError::Internal(format!("unknown grant source {other}"))),
    }
}

fn event_type_str(event_type: &AdsEventType) -> &'static str {
    match event_type {
        AdsEventType::GrantCreated => "grant.created",
        AdsEventType::GrantRevoked => "grant.revoked",
        AdsEventType::RequestCreated => "request.created",
        AdsEventType::RequestApproved => "request.approved",
        AdsEventType::RequestRejected => "request.rejected",
    }
}

fn parse_event_type(raw: &str) -> Result<AdsEventType, AdsError> {
    match raw {
        "grant.created" => Ok(AdsEventType::GrantCreated),
        "grant.revoked" => Ok(AdsEventType::GrantRevoked),
        "request.created" => Ok(AdsEventType::RequestCreated),
        "request.approved" => Ok(AdsEventType::RequestApproved),
        "request.rejected" => Ok(AdsEventType::RequestRejected),
        other => Err(AdsError::Internal(format!("unknown event type {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ga4gh_types::DuoCode;

    async fn test_store() -> AdsStore {
        AdsStore::connect(
            &DatabaseConfig {
                driver: DatabaseDriver::Sqlite,
                url: Some("sqlite::memory:".to_string()),
                url_env: "ADS_DATABASE_URL".to_string(),
                auto_migrate: true,
            },
            "sqlite::memory:",
            vec![],
        )
        .await
        .expect("memory store")
    }

    #[tokio::test]
    async fn dataset_and_grant_lifecycle() {
        let store = test_store().await;
        let dataset = store
            .create_dataset(&CreateDatasetRequest {
                name: "Test dataset".to_string(),
                description: None,
                duo_codes: vec![DuoCode::Gru],
                external_id: Some("drs:abc".to_string()),
                auto_approve_enabled: true,
                auto_approve_threshold: 100,
                dac_group: None,
            })
            .await
            .expect("create dataset");

        let project = store
            .create_project(&CreateProjectRequest {
                researcher_id: "researcher@example.org".to_string(),
                name: "Project".to_string(),
                description: None,
                duo_codes: vec![DuoCode::Gru],
            })
            .await
            .expect("create project");

        let evaluation = crate::duo::evaluate_duo_codes(
            &dataset.duo_codes,
            &project.duo_codes,
            dataset.auto_approve_threshold,
        );
        let request = store
            .create_access_request(
                &CreateAccessRequestBody {
                    researcher_id: "researcher@example.org".to_string(),
                    dataset_id: dataset.id,
                    project_id: project.id,
                    justification: None,
                },
                Some(evaluation),
            )
            .await
            .expect("create request");

        assert_eq!(request.status, AccessRequestStatus::Approved);
        let grants = store
            .list_grants(Some("researcher@example.org"), None)
            .await
            .expect("list grants");
        assert_eq!(grants.len(), 1);
    }
}
