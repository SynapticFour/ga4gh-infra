// SPDX-License-Identifier: Apache-2.0

//! PostgreSQL and SQLite persistence for registered GA4GH services.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use ga4gh_types::ServiceType;
use sqlx::SqlitePool;
use tracing::instrument;

use crate::config::{DatabaseConfig, DatabaseDriver};
use crate::error::RegistryError;
use crate::types::ExternalService;

#[cfg(feature = "postgres")]
use sqlx::PgPool;

#[derive(Clone)]
enum DbPool {
    #[cfg(feature = "postgres")]
    Postgres(PgPool),
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
}

/// Database-backed service registry store.
#[derive(Clone)]
pub struct ServiceStore {
    pool: DbPool,
}

impl ServiceStore {
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
                "service-registry was built without the `postgres` feature".to_string(),
            )),
            #[cfg(not(feature = "sqlite"))]
            DatabaseDriver::Sqlite => Err(RegistryError::Config(
                "service-registry was built without the `sqlite` feature".to_string(),
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

    #[cfg(test)]
    pub(crate) fn from_pool_postgres(pool: PgPool) -> Self {
        Self {
            pool: DbPool::Postgres(pool),
        }
    }

    #[cfg(all(test, feature = "sqlite"))]
    #[allow(dead_code)]
    pub(crate) fn from_pool_sqlite(pool: SqlitePool) -> Self {
        Self {
            pool: DbPool::Sqlite(pool),
        }
    }

    /// Register or update a service entry keyed by service id.
    #[instrument(skip(self, service))]
    pub async fn upsert(&self, service: &ExternalService) -> Result<(), RegistryError> {
        service.validate()?;
        let now = unix_now();
        let payload = serde_json::to_string(service)
            .map_err(|err| RegistryError::Internal(err.to_string()))?;

        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO registered_services (id, url, service_info, registered_at, updated_at)
                    VALUES ($1, $2, $3, $4, $4)
                    ON CONFLICT (id) DO UPDATE SET
                        url = EXCLUDED.url,
                        service_info = EXCLUDED.service_info,
                        updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(&service.info.id)
                .bind(&service.url)
                .bind(payload)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO registered_services (id, url, service_info, registered_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?4)
                    ON CONFLICT (id) DO UPDATE SET
                        url = excluded.url,
                        service_info = excluded.service_info,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&service.info.id)
                .bind(&service.url)
                .bind(payload)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;
            }
        }

        Ok(())
    }

    /// Remove a service from the registry.
    #[instrument(skip(self))]
    pub async fn delete(&self, service_id: &str) -> Result<(), RegistryError> {
        let rows_affected = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => sqlx::query("DELETE FROM registered_services WHERE id = $1")
                .bind(service_id)
                .execute(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?
                .rows_affected(),
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM registered_services WHERE id = ?1")
                .bind(service_id)
                .execute(pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?
                .rows_affected(),
        };

        if rows_affected == 0 {
            return Err(RegistryError::NotFound);
        }
        Ok(())
    }

    /// List all registered services.
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<ExternalService>, RegistryError> {
        let rows: Vec<(String,)> = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query_as("SELECT service_info FROM registered_services ORDER BY id ASC")
                    .fetch_all(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))?
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query_as("SELECT service_info FROM registered_services ORDER BY id ASC")
                    .fetch_all(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))?
            }
        };

        rows.into_iter()
            .map(|(value,)| decode_service(value))
            .collect()
    }

    /// Fetch a single registered service by id.
    #[instrument(skip(self))]
    pub async fn get(&self, service_id: &str) -> Result<ExternalService, RegistryError> {
        let row: Option<(String,)> = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query_as("SELECT service_info FROM registered_services WHERE id = $1")
                    .bind(service_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))?
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query_as("SELECT service_info FROM registered_services WHERE id = ?1")
                    .bind(service_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|err| RegistryError::Database(err.to_string()))?
            }
        };

        let Some((value,)) = row else {
            return Err(RegistryError::NotFound);
        };
        decode_service(value)
    }

    /// List distinct service types across all registered services.
    #[instrument(skip(self))]
    pub async fn list_types(&self) -> Result<Vec<ServiceType>, RegistryError> {
        let services = self.list().await?;
        let mut seen = HashSet::new();
        let mut types = Vec::new();

        for service in services {
            let key = (
                service.info.r#type.group.clone(),
                service.info.r#type.artifact.clone(),
                service.info.r#type.version.clone(),
            );
            if seen.insert(key) {
                types.push(service.info.r#type);
            }
        }

        types.sort_by(|a, b| {
            a.group
                .cmp(&b.group)
                .then_with(|| a.artifact.cmp(&b.artifact))
                .then_with(|| a.version.cmp(&b.version))
        });

        Ok(types)
    }
}

fn decode_service(value: String) -> Result<ExternalService, RegistryError> {
    serde_json::from_str(&value).map_err(|err| RegistryError::Database(err.to_string()))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};

    use super::*;
    use crate::types::ExternalService;

    fn database_url() -> Option<String> {
        std::env::var("TEST_DATABASE_URL")
            .ok()
            .or_else(|| std::env::var("SERVICE_REGISTRY_DATABASE_URL").ok())
    }

    fn sample_service(id: &str) -> ExternalService {
        ExternalService {
            info: ServiceInfo {
                id: id.to_string(),
                name: format!("Service {id}"),
                r#type: ServiceType {
                    group: "org.ga4gh".to_string(),
                    artifact: "passport".to_string(),
                    version: "1.2".to_string(),
                },
                organization: ServiceOrganization {
                    name: "Example".to_string(),
                    url: "https://example.org".to_string(),
                    contact_url: None,
                },
                version: "0.1.0".to_string(),
                description: None,
                documentation_url: None,
                created_at: None,
                updated_at: None,
                environment: Some("test".to_string()),
            },
            url: format!("https://{id}.example.org"),
        }
    }

    #[tokio::test]
    async fn upserts_lists_and_deletes_services() {
        let Some(database_url) = database_url() else {
            return;
        };

        let database = DatabaseConfig {
            driver: DatabaseDriver::Postgres,
            url: Some(database_url),
            url_env: "SERVICE_REGISTRY_DATABASE_URL".to_string(),
            auto_migrate: true,
        };
        let store = ServiceStore::connect(&database, database.url.as_ref().unwrap())
            .await
            .expect("connect");
        let mut service = sample_service("org.example.test-service");

        store.upsert(&service).await.expect("upsert");
        let listed = store.list().await.expect("list");
        assert!(listed.iter().any(|entry| entry.info.id == service.info.id));

        service.info.version = "0.2.0".to_string();
        store.upsert(&service).await.expect("update");
        let fetched = store.get(&service.info.id).await.expect("get");
        assert_eq!(fetched.info.version, "0.2.0");

        let types = store.list_types().await.expect("types");
        assert!(types.iter().any(|ty| ty.artifact == "passport"));

        store.delete(&service.info.id).await.expect("delete");
        assert!(store.get(&service.info.id).await.is_err());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sqlite_round_trips_services() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("service_registry.sqlite");
        let url = format!("sqlite://{}", path.display());
        let database = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: Some(url.clone()),
            url_env: "SERVICE_REGISTRY_DATABASE_URL".to_string(),
            auto_migrate: true,
        };

        let store = ServiceStore::connect(&database, &url)
            .await
            .expect("connect");
        let service = sample_service("org.example.sqlite-service");
        store.upsert(&service).await.expect("upsert");

        let fetched = store.get(&service.info.id).await.expect("get");
        assert_eq!(fetched.info.id, service.info.id);

        let types = store.list_types().await.expect("types");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].artifact, "passport");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sqlite_creates_parent_directory_for_file_database() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("service_registry.sqlite");
        let url = format!("sqlite://{}", path.display());
        let database = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: Some(url.clone()),
            url_env: "SERVICE_REGISTRY_DATABASE_URL".to_string(),
            auto_migrate: true,
        };

        ServiceStore::connect(&database, &url)
            .await
            .expect("connect");
        assert!(path.exists());
    }
}
