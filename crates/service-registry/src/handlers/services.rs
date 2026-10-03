// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Registry CRUD handlers.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use ga4gh_types::ServiceType;
use serde::Deserialize;
use tracing::instrument;

use crate::app::AppState;
use crate::auth::verify_registration_key;
use crate::error::RegistryError;
use crate::store::unix_now;
use crate::types::{present_service, ExternalService, ServiceView};

/// Query parameters for `GET /services`.
#[derive(Debug, Default, Deserialize)]
pub struct ListServicesQuery {
    /// Exact `type.artifact` match. Absent returns every row.
    #[serde(default, rename = "type")]
    pub type_artifact: Option<String>,
}

/// List all registered GA4GH services.
#[instrument(skip(state))]
pub async fn list_services(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListServicesQuery>,
) -> Result<Json<Vec<ServiceView>>, RegistryError> {
    let now = unix_now();
    let age = state.config.server.stale_after_seconds;
    let mut views = Vec::new();
    for stored in state.store.list().await? {
        if let Some(artifact) = query.type_artifact.as_deref() {
            if stored.service.info.r#type.artifact != artifact {
                continue;
            }
        }
        views.push(present_service(stored, now, age));
    }
    Ok(Json(views))
}

/// Fetch a registered service by id.
#[instrument(skip(state))]
pub async fn get_service(
    State(state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> Result<Json<ServiceView>, RegistryError> {
    let stored = state.store.get(&service_id).await?;
    Ok(Json(present_service(
        stored,
        unix_now(),
        state.config.server.stale_after_seconds,
    )))
}

/// List distinct service types present in the registry.
#[instrument(skip(state))]
pub async fn list_service_types(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ServiceType>>, RegistryError> {
    let types = state.store.list_types().await?;
    Ok(Json(types))
}

/// Register or update a service (internal, authenticated).
#[instrument(skip(state, headers, body))]
pub async fn register_service(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ExternalService>,
) -> Result<StatusCode, RegistryError> {
    ensure_writable(&state)?;
    ensure_registration_authorized(&state, &headers)?;
    body.validate()?;
    state.store.upsert(&body).await?;
    tracing::info!(service_id = %body.info.id, url = %body.url, "service registered");
    Ok(StatusCode::NO_CONTENT)
}

/// Remove a registered service (internal, authenticated).
#[instrument(skip(state, headers))]
pub async fn delete_service(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(service_id): Path<String>,
) -> Result<StatusCode, RegistryError> {
    ensure_writable(&state)?;
    ensure_registration_authorized(&state, &headers)?;
    state.store.delete(&service_id).await?;
    tracing::info!(service_id = %service_id, "service deregistered");
    Ok(StatusCode::NO_CONTENT)
}

fn ensure_writable(state: &AppState) -> Result<(), RegistryError> {
    if state.config.server.read_only {
        return Err(RegistryError::ReadOnly);
    }
    Ok(())
}

fn ensure_registration_authorized(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), RegistryError> {
    let presented = headers
        .get("X-API-Key")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|value| value.to_str().ok())
        .ok_or(RegistryError::Unauthorized)?;

    let expected = state
        .registration_key
        .as_deref()
        .ok_or(RegistryError::Config(
            "registration API key is not configured".to_string(),
        ))?;

    if verify_registration_key(presented, expected) {
        Ok(())
    } else {
        Err(RegistryError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, DatabaseConfig, DatabaseDriver, RegistryConfig, ServerConfig};
    use crate::store::ServiceStore;
    use sqlx::PgPool;

    fn headers_with_key(key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", key.parse().expect("header"));
        headers
    }

    fn test_state(read_only: bool, key: Option<&str>) -> AppState {
        AppState {
            config: RegistryConfig {
                server: ServerConfig {
                    host: "127.0.0.1".to_string(),
                    port: 8083,
                    external_url: "https://registry.example.org".to_string(),
                    environment: "test".to_string(),
                    read_only,
                    stale_after_seconds: None,
                },
                database: DatabaseConfig {
                    driver: DatabaseDriver::Postgres,
                    url: None,
                    url_env: "SERVICE_REGISTRY_DATABASE_URL".to_string(),
                    auto_migrate: false,
                },
                auth: AuthConfig {
                    registration_api_key_env: "SERVICE_REGISTRY_REGISTRATION_KEY".to_string(),
                },
            },
            store: ServiceStore::from_pool_postgres(
                PgPool::connect_lazy("postgres://unused").expect("lazy pool"),
            ),
            registration_key: key.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn read_only_mode_rejects_writes() {
        let state = test_state(true, Some("secret"));
        assert!(matches!(
            ensure_writable(&state),
            Err(RegistryError::ReadOnly)
        ));
    }

    #[tokio::test]
    async fn registration_requires_matching_api_key() {
        let state = test_state(false, Some("secret"));
        assert!(ensure_registration_authorized(&state, &headers_with_key("secret")).is_ok());
        assert!(ensure_registration_authorized(&state, &headers_with_key("wrong")).is_err());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn get_uses_server_time_filters_type_and_marks_stale_without_deleting() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("registry.sqlite");
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
        let mut wes = sample_listed("org.example.wes", "wes");
        wes.info.updated_at = Some("1999-01-01T00:00:00Z".to_string());
        let drs = sample_listed("org.example.drs", "drsservice");
        store.upsert(&wes).await.expect("upsert wes");
        store.upsert(&drs).await.expect("upsert drs");
        let first = store.get(&wes.info.id).await.expect("get").updated_at;
        wes.url = "https://wes-2.example.org".to_string();
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        store.upsert(&wes).await.expect("reupsert");
        let second = store.get(&wes.info.id).await.expect("get").updated_at;
        assert!(second > first, "re-POST must move the server timestamp");

        let mut config = test_state(false, Some("secret")).config;
        config.server.stale_after_seconds = Some(1);
        let state = Arc::new(AppState {
            config,
            store,
            registration_key: Some("secret".to_string()),
        });
        let Json(filtered) = list_services(
            State(Arc::clone(&state)),
            Query(ListServicesQuery {
                type_artifact: Some("wes".to_string()),
            }),
        )
        .await
        .expect("list");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].service.url, "https://wes-2.example.org");
        assert_ne!(
            filtered[0].service.info.updated_at.as_deref(),
            Some("1999-01-01T00:00:00Z")
        );
        assert!(!filtered[0].stale, "a re-POST inside the age is not stale");
        let Json(all) = list_services(
            State(Arc::clone(&state)),
            Query(ListServicesQuery {
                type_artifact: None,
            }),
        )
        .await
        .expect("list all");
        assert_eq!(all.len(), 2, "stale rows stay until DELETE");
        let stored = state.store.get("org.example.drs").await.expect("drs row");
        let marked = present_service(stored, second + 5, Some(1));
        assert!(
            marked.stale,
            "a row older than stale_after_seconds is marked"
        );
    }

    #[cfg(feature = "sqlite")]
    fn sample_listed(id: &str, artifact: &str) -> ExternalService {
        use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};
        ExternalService {
            info: ServiceInfo {
                id: id.to_string(),
                name: id.to_string(),
                r#type: ServiceType {
                    group: "org.ga4gh".to_string(),
                    artifact: artifact.to_string(),
                    version: "1.0.0".to_string(),
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
            url: format!("https://{artifact}.example.org"),
        }
    }
}
