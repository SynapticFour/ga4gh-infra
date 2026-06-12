// SPDX-License-Identifier: Apache-2.0

//! PostgreSQL persistence for registered GA4GH services.

use std::time::{SystemTime, UNIX_EPOCH};

use ga4gh_types::ServiceType;
use sqlx::PgPool;
use tracing::instrument;

use crate::error::RegistryError;
use crate::types::ExternalService;

/// PostgreSQL-backed service registry store.
#[derive(Clone)]
pub struct ServiceStore {
    pool: PgPool,
}

impl ServiceStore {
    /// Connect to PostgreSQL and run pending migrations.
    pub async fn connect(database_url: &str) -> Result<Self, RegistryError> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|err| RegistryError::Database(err.to_string()))?;
        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|err| RegistryError::Database(err.to_string()))?;
        Ok(Self { pool })
    }

    #[cfg(test)]
    pub(crate) fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Register or update a service entry keyed by service id.
    #[instrument(skip(self, service))]
    pub async fn upsert(&self, service: &ExternalService) -> Result<(), RegistryError> {
        service.validate()?;
        let now = unix_now();
        let payload = serde_json::to_value(service)
            .map_err(|err| RegistryError::Internal(err.to_string()))?;

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
        .execute(&self.pool)
        .await
        .map_err(|err| RegistryError::Database(err.to_string()))?;

        Ok(())
    }

    /// Remove a service from the registry.
    #[instrument(skip(self))]
    pub async fn delete(&self, service_id: &str) -> Result<(), RegistryError> {
        let result = sqlx::query("DELETE FROM registered_services WHERE id = $1")
            .bind(service_id)
            .execute(&self.pool)
            .await
            .map_err(|err| RegistryError::Database(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RegistryError::NotFound);
        }
        Ok(())
    }

    /// List all registered services.
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<ExternalService>, RegistryError> {
        let rows: Vec<(serde_json::Value,)> =
            sqlx::query_as("SELECT service_info FROM registered_services ORDER BY id ASC")
                .fetch_all(&self.pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;

        rows.into_iter()
            .map(|(value,)| decode_service(value))
            .collect()
    }

    /// Fetch a single registered service by id.
    #[instrument(skip(self))]
    pub async fn get(&self, service_id: &str) -> Result<ExternalService, RegistryError> {
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT service_info FROM registered_services WHERE id = $1")
                .bind(service_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|err| RegistryError::Database(err.to_string()))?;

        let Some((value,)) = row else {
            return Err(RegistryError::NotFound);
        };
        decode_service(value)
    }

    /// List distinct service types across all registered services.
    #[instrument(skip(self))]
    pub async fn list_types(&self) -> Result<Vec<ServiceType>, RegistryError> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT service_info->'type'
            FROM registered_services
            ORDER BY service_info->'type'->>'group',
                     service_info->'type'->>'artifact',
                     service_info->'type'->>'version'
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|err| RegistryError::Database(err.to_string()))?;

        rows.into_iter()
            .map(|(value,)| {
                serde_json::from_value(value)
                    .map_err(|err| RegistryError::Database(err.to_string()))
            })
            .collect()
    }
}

fn decode_service(value: serde_json::Value) -> Result<ExternalService, RegistryError> {
    serde_json::from_value(value).map_err(|err| RegistryError::Database(err.to_string()))
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

        let store = ServiceStore::connect(&database_url).await.expect("connect");
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
}
