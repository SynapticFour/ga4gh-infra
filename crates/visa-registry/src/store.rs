// SPDX-License-Identifier: Apache-2.0

//! PostgreSQL and SQLite persistence for visa assertions and DAC API keys.

use std::time::{SystemTime, UNIX_EPOCH};

use ga4gh_types::{VisaAuthority, VisaConditions, VisaType};
use sqlx::{Row, SqlitePool};
use tracing::instrument;
use uuid::Uuid;

use crate::auth::{hash_api_key, verify_api_key};
use crate::config::{DatabaseConfig, DatabaseDriver};
use crate::error::RegistryError;

#[cfg(feature = "postgres")]
use sqlx::PgPool;

#[derive(Clone)]
enum DbPool {
    #[cfg(feature = "postgres")]
    Postgres(PgPool),
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
}

/// Database-backed visa assertion store.
#[derive(Clone)]
pub struct VisaStore {
    pool: DbPool,
}

/// Unsigned visa assertion stored in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisaAssertion {
    /// Stable assertion identifier.
    pub id: Uuid,
    /// Researcher subject identifier.
    pub sub: String,
    /// GA4GH visa type.
    pub visa_type: VisaType,
    /// Assertion value.
    pub value: String,
    /// Source organization URL.
    pub source: String,
    /// Optional authority level within the source organization.
    pub by: Option<VisaAuthority>,
    /// Optional visa conditions in DNF form.
    pub conditions: Option<VisaConditions>,
    /// When the source organization made the assertion (Unix seconds).
    pub asserted: i64,
    /// When the assertion row was created (Unix seconds).
    pub created_at: i64,
    /// When the assertion was revoked, if applicable.
    pub revoked_at: Option<i64>,
    /// Optional expiry timestamp for the assertion (Unix seconds).
    pub expires_at: Option<i64>,
}

/// Input for creating a new unsigned visa assertion.
#[derive(Debug, Clone)]
pub struct NewVisaAssertion {
    /// Researcher subject identifier.
    pub sub: String,
    /// GA4GH visa type.
    pub visa_type: VisaType,
    /// Assertion value.
    pub value: String,
    /// Source organization URL.
    pub source: String,
    /// Optional authority level within the source organization.
    pub by: Option<VisaAuthority>,
    /// Optional visa conditions in DNF form.
    pub conditions: Option<VisaConditions>,
    /// When the source organization made the assertion (Unix seconds).
    pub asserted: i64,
    /// Optional expiry timestamp for the assertion (Unix seconds).
    pub expires_at: Option<i64>,
}

impl VisaStore {
    /// Connect using the configured driver and run migrations when enabled.
    pub async fn connect(database: &DatabaseConfig, url: &str) -> Result<Self, RegistryError> {
        match database.driver {
            #[cfg(feature = "postgres")]
            DatabaseDriver::Postgres => {
                let pool = PgPool::connect(url)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))?;
                if database.auto_migrate {
                    sqlx::migrate!()
                        .run(&pool)
                        .await
                        .map_err(|err| RegistryError::Database(err.to_string()))?;
                }
                Ok(Self {
                    pool: DbPool::Postgres(pool),
                })
            }
            #[cfg(feature = "sqlite")]
            DatabaseDriver::Sqlite => Self::connect_sqlite(url).await,
            #[cfg(not(feature = "postgres"))]
            DatabaseDriver::Postgres => Err(RegistryError::Config(
                "visa-registry was built without the `postgres` feature".to_string(),
            )),
            #[cfg(not(feature = "sqlite"))]
            DatabaseDriver::Sqlite => Err(RegistryError::Config(
                "visa-registry was built without the `sqlite` feature".to_string(),
            )),
        }
    }

    #[cfg(feature = "sqlite")]
    async fn connect_sqlite(url: &str) -> Result<Self, RegistryError> {
        use std::str::FromStr;

        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

        let options = SqliteConnectOptions::from_str(url)
            .map_err(|err| RegistryError::Database(format!("invalid SQLite URL: {err}")))?
            .create_if_missing(true);

        if !options.get_filename().as_os_str().is_empty() {
            if let Some(parent) = options.get_filename().parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|err| {
                        RegistryError::Database(format!(
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
            .map_err(|err| RegistryError::Database(err.to_string()))?;

        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|err| RegistryError::Database(err.to_string()))?;

        Ok(Self {
            pool: DbPool::Sqlite(pool),
        })
    }

    /// Register a bootstrap API key when the database has no active keys.
    pub async fn ensure_bootstrap_api_key(
        &self,
        raw_key: &str,
        name: &str,
    ) -> Result<(), RegistryError> {
        let active_count = self.count_active_api_keys().await?;
        if active_count > 0 {
            return Ok(());
        }
        self.insert_api_key(name, raw_key).await
    }

    /// Insert a new hashed API key.
    pub async fn insert_api_key(&self, name: &str, raw_key: &str) -> Result<(), RegistryError> {
        let now = unix_now();
        let id = Uuid::new_v4().to_string();
        let key_hash = hash_api_key(raw_key);

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
                .map_err(|err| RegistryError::Database(err.to_string()))?;
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
                .map_err(|err| RegistryError::Database(err.to_string()))?;
            }
        }

        Ok(())
    }

    /// Verify a raw API key against active hashed keys in the database.
    #[instrument(skip(self, raw_key))]
    pub async fn verify_api_key(&self, raw_key: &str) -> Result<(), RegistryError> {
        let authorized = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query("SELECT key_hash FROM api_keys WHERE revoked_at IS NULL")
                    .fetch_all(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))?;
                rows.iter().any(|row| {
                    let stored_hash: String = row.get("key_hash");
                    verify_api_key(raw_key, &stored_hash)
                })
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT key_hash FROM api_keys WHERE revoked_at IS NULL")
                    .fetch_all(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))?;
                rows.iter().any(|row| {
                    let stored_hash: String = row.get("key_hash");
                    verify_api_key(raw_key, &stored_hash)
                })
            }
        };

        if authorized {
            Ok(())
        } else {
            Err(RegistryError::Unauthorized)
        }
    }

    /// Create a new unsigned visa assertion.
    #[instrument(skip(self, input))]
    pub async fn create_assertion(
        &self,
        input: NewVisaAssertion,
    ) -> Result<VisaAssertion, RegistryError> {
        let id = Uuid::new_v4();
        let now = unix_now();
        let conditions = input
            .conditions
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|err| RegistryError::BadRequest(err.to_string()))?;
        let by_authority = input.by.map(authority_to_db);

        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO visa_assertions (
                        id, sub, visa_type, value, source, by_authority, conditions,
                        asserted, created_at, expires_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    "#,
                )
                .bind(id.to_string())
                .bind(&input.sub)
                .bind(input.visa_type.as_str())
                .bind(&input.value)
                .bind(&input.source)
                .bind(by_authority)
                .bind(conditions)
                .bind(input.asserted)
                .bind(now)
                .bind(input.expires_at)
                .execute(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO visa_assertions (
                        id, sub, visa_type, value, source, by_authority, conditions,
                        asserted, created_at, expires_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    "#,
                )
                .bind(id.to_string())
                .bind(&input.sub)
                .bind(input.visa_type.as_str())
                .bind(&input.value)
                .bind(&input.source)
                .bind(by_authority)
                .bind(conditions)
                .bind(input.asserted)
                .bind(now)
                .bind(input.expires_at)
                .execute(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;
            }
        }

        Ok(VisaAssertion {
            id,
            sub: input.sub,
            visa_type: input.visa_type,
            value: input.value,
            source: input.source,
            by: input.by,
            conditions: input.conditions,
            asserted: input.asserted,
            created_at: now,
            revoked_at: None,
            expires_at: input.expires_at,
        })
    }

    /// Revoke a visa assertion by identifier.
    #[instrument(skip(self))]
    pub async fn revoke_assertion(&self, id: Uuid) -> Result<(), RegistryError> {
        let now = unix_now();
        let affected = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => sqlx::query(
                "UPDATE visa_assertions SET revoked_at = $1 WHERE id = $2 AND revoked_at IS NULL",
            )
            .bind(now)
            .bind(id.to_string())
            .execute(pool)
            .await
            .map_err(|err| RegistryError::Database(err.to_string()))?
            .rows_affected(),
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => sqlx::query(
                "UPDATE visa_assertions SET revoked_at = $1 WHERE id = $2 AND revoked_at IS NULL",
            )
            .bind(now)
            .bind(id.to_string())
            .execute(pool)
            .await
            .map_err(|err| RegistryError::Database(err.to_string()))?
            .rows_affected(),
        };

        if affected == 0 {
            return Err(RegistryError::NotFound);
        }
        Ok(())
    }

    /// List active visa assertions for a researcher subject.
    #[instrument(skip(self))]
    pub async fn list_active_for_sub(
        &self,
        sub: &str,
    ) -> Result<Vec<VisaAssertion>, RegistryError> {
        let now = unix_now();
        let assertions = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT id, sub, visa_type, value, source, by_authority, conditions,
                           asserted, created_at, revoked_at, expires_at
                    FROM visa_assertions
                    WHERE sub = $1
                      AND revoked_at IS NULL
                      AND (expires_at IS NULL OR expires_at > $2)
                    ORDER BY created_at ASC
                    "#,
                )
                .bind(sub)
                .bind(now)
                .fetch_all(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;
                rows.iter()
                    .map(row_to_assertion_postgres)
                    .collect::<Result<Vec<_>, _>>()?
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT id, sub, visa_type, value, source, by_authority, conditions,
                           asserted, created_at, revoked_at, expires_at
                    FROM visa_assertions
                    WHERE sub = $1
                      AND revoked_at IS NULL
                      AND (expires_at IS NULL OR expires_at > $2)
                    ORDER BY created_at ASC
                    "#,
                )
                .bind(sub)
                .bind(now)
                .fetch_all(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;
                rows.iter()
                    .map(row_to_assertion_sqlite)
                    .collect::<Result<Vec<_>, _>>()?
            }
        };

        Ok(assertions)
    }

    async fn count_active_api_keys(&self) -> Result<i64, RegistryError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE revoked_at IS NULL")
                    .fetch_one(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE revoked_at IS NULL")
                    .fetch_one(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))
            }
        }
    }
}

#[derive(Debug)]
struct AssertionRow {
    id: String,
    sub: String,
    visa_type: String,
    value: String,
    source: String,
    by_authority: Option<String>,
    conditions: Option<String>,
    asserted: i64,
    created_at: i64,
    revoked_at: Option<i64>,
    expires_at: Option<i64>,
}

#[cfg(feature = "postgres")]
fn row_to_assertion_postgres(row: &sqlx::postgres::PgRow) -> Result<VisaAssertion, RegistryError> {
    row_to_assertion(AssertionRow {
        id: row.get("id"),
        sub: row.get("sub"),
        visa_type: row.get("visa_type"),
        value: row.get("value"),
        source: row.get("source"),
        by_authority: row.get("by_authority"),
        conditions: row.get("conditions"),
        asserted: row.get("asserted"),
        created_at: row.get("created_at"),
        revoked_at: row.get("revoked_at"),
        expires_at: row.get("expires_at"),
    })
}

#[cfg(feature = "sqlite")]
fn row_to_assertion_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<VisaAssertion, RegistryError> {
    row_to_assertion(AssertionRow {
        id: row.get("id"),
        sub: row.get("sub"),
        visa_type: row.get("visa_type"),
        value: row.get("value"),
        source: row.get("source"),
        by_authority: row.get("by_authority"),
        conditions: row.get("conditions"),
        asserted: row.get("asserted"),
        created_at: row.get("created_at"),
        revoked_at: row.get("revoked_at"),
        expires_at: row.get("expires_at"),
    })
}

fn row_to_assertion(row: AssertionRow) -> Result<VisaAssertion, RegistryError> {
    let visa_type = row
        .visa_type
        .parse::<VisaType>()
        .map_err(|err| RegistryError::Database(err.0))?;

    let by = row
        .by_authority
        .map(|value| authority_from_db(&value))
        .transpose()
        .map_err(RegistryError::Database)?;

    let conditions = row
        .conditions
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(|err| RegistryError::Database(err.to_string()))?;

    let id = row
        .id
        .parse::<Uuid>()
        .map_err(|err| RegistryError::Database(err.to_string()))?;

    Ok(VisaAssertion {
        id,
        sub: row.sub,
        visa_type,
        value: row.value,
        source: row.source,
        by,
        conditions,
        asserted: row.asserted,
        created_at: row.created_at,
        revoked_at: row.revoked_at,
        expires_at: row.expires_at,
    })
}

fn authority_to_db(authority: VisaAuthority) -> String {
    match authority {
        VisaAuthority::Self_ => "self".to_string(),
        VisaAuthority::Peer => "peer".to_string(),
        VisaAuthority::System => "system".to_string(),
        VisaAuthority::So => "so".to_string(),
        VisaAuthority::Dac => "dac".to_string(),
    }
}

fn authority_from_db(raw: &str) -> Result<VisaAuthority, String> {
    match raw {
        "self" => Ok(VisaAuthority::Self_),
        "peer" => Ok(VisaAuthority::Peer),
        "system" => Ok(VisaAuthority::System),
        "so" => Ok(VisaAuthority::So),
        "dac" => Ok(VisaAuthority::Dac),
        other => Err(format!("invalid stored visa authority: {other}")),
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn postgres_url() -> Option<String> {
        std::env::var("TEST_DATABASE_URL")
            .ok()
            .or_else(|| std::env::var("REGISTRY_DATABASE_URL").ok())
    }

    async fn sqlite_store() -> VisaStore {
        let database = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: Some("sqlite::memory:".to_string()),
            url_env: "UNUSED".to_string(),
            auto_migrate: true,
        };
        VisaStore::connect(&database, "sqlite::memory:")
            .await
            .expect("sqlite connect")
    }

    async fn exercise_assertion_lifecycle(store: &VisaStore) {
        let sub = format!("researcher-{}", Uuid::new_v4());

        let created = store
            .create_assertion(NewVisaAssertion {
                sub: sub.clone(),
                visa_type: VisaType::ControlledAccessGrants,
                value: "dataset-test".to_string(),
                source: "https://dac.example.org".to_string(),
                by: Some(VisaAuthority::Dac),
                conditions: None,
                asserted: unix_now(),
                expires_at: None,
            })
            .await
            .expect("create");

        let active = store.list_active_for_sub(&sub).await.expect("list");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, created.id);

        store.revoke_assertion(created.id).await.expect("revoke");
        let active = store
            .list_active_for_sub(&sub)
            .await
            .expect("list after revoke");
        assert!(active.is_empty());
    }

    async fn exercise_bootstrap_key(store: &VisaStore) {
        let raw_key = format!("bootstrap-{}", Uuid::new_v4());
        store
            .ensure_bootstrap_api_key(&raw_key, "bootstrap")
            .await
            .expect("bootstrap first");
        store
            .ensure_bootstrap_api_key("ignored-key", "bootstrap")
            .await
            .expect("bootstrap second");
        store.verify_api_key(&raw_key).await.expect("valid key");
        assert!(store.verify_api_key("wrong-key").await.is_err());
    }

    #[tokio::test]
    async fn sqlite_round_trips_assertion_lifecycle() {
        exercise_assertion_lifecycle(&sqlite_store().await).await;
    }

    #[tokio::test]
    async fn sqlite_bootstrap_api_key_is_registered_once() {
        exercise_bootstrap_key(&sqlite_store().await).await;
    }

    #[tokio::test]
    async fn sqlite_creates_parent_directory_for_file_database() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("nested/visa_registry.sqlite");
        let url = format!("sqlite://{}", db_path.display());
        let database = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: Some(url.clone()),
            url_env: "UNUSED".to_string(),
            auto_migrate: true,
        };

        VisaStore::connect(&database, &url)
            .await
            .expect("connect file sqlite");
        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn postgres_round_trips_assertion_lifecycle() {
        let Some(database_url) = postgres_url() else {
            return;
        };
        let database = DatabaseConfig {
            driver: DatabaseDriver::Postgres,
            url: Some(database_url.clone()),
            url_env: "REGISTRY_DATABASE_URL".to_string(),
            auto_migrate: true,
        };
        let store = VisaStore::connect(&database, &database_url)
            .await
            .expect("connect postgres");
        exercise_assertion_lifecycle(&store).await;
    }

    #[tokio::test]
    async fn postgres_bootstrap_api_key_is_registered_once() {
        let Some(database_url) = postgres_url() else {
            return;
        };
        let database = DatabaseConfig {
            driver: DatabaseDriver::Postgres,
            url: Some(database_url.clone()),
            url_env: "REGISTRY_DATABASE_URL".to_string(),
            auto_migrate: true,
        };
        let store = VisaStore::connect(&database, &database_url)
            .await
            .expect("connect postgres");
        exercise_bootstrap_key(&store).await;
    }
}
