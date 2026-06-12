// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Registry CRUD handlers.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use ga4gh_types::ServiceType;
use tracing::instrument;

use crate::app::AppState;
use crate::auth::verify_registration_key;
use crate::error::RegistryError;
use crate::types::ExternalService;

/// List all registered GA4GH services.
#[instrument(skip(state))]
pub async fn list_services(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ExternalService>>, RegistryError> {
    let services = state.store.list().await?;
    Ok(Json(services))
}

/// Fetch a registered service by id.
#[instrument(skip(state))]
pub async fn get_service(
    State(state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> Result<Json<ExternalService>, RegistryError> {
    let service = state.store.get(&service_id).await?;
    Ok(Json(service))
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
    use crate::config::{AuthConfig, DatabaseConfig, RegistryConfig, ServerConfig};
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
                },
                database: DatabaseConfig {
                    url_env: "SERVICE_REGISTRY_DATABASE_URL".to_string(),
                },
                auth: AuthConfig {
                    registration_api_key_env: "SERVICE_REGISTRY_REGISTRATION_KEY".to_string(),
                },
            },
            store: ServiceStore::from_pool(
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
}
