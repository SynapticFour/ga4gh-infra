// SPDX-License-Identifier: Apache-2.0

//! Registry service-info handler.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};
use tracing::instrument;

use crate::app::AppState;

/// Return GA4GH Service Info metadata for this registry.
#[instrument(skip(state))]
pub async fn service_info(State(state): State<Arc<AppState>>) -> Json<ServiceInfo> {
    Json(ServiceInfo {
        id: format!(
            "{}.service-registry",
            state.config.external_url().replace("https://", "")
        ),
        name: "GA4GH Service Registry".to_string(),
        r#type: ServiceType {
            group: "org.ga4gh".to_string(),
            artifact: "service-registry".to_string(),
            version: "1.0.0".to_string(),
        },
        organization: ServiceOrganization {
            name: "GA4GH Infra".to_string(),
            url: state.config.external_url().to_string(),
            contact_url: None,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: Some(
            "Registry of GA4GH services for discovery across organizational boundaries".to_string(),
        ),
        documentation_url: None,
        created_at: None,
        updated_at: None,
        environment: Some(state.config.server.environment.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, DatabaseConfig, RegistryConfig, ServerConfig};

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            config: RegistryConfig {
                server: ServerConfig {
                    host: "127.0.0.1".to_string(),
                    port: 8083,
                    external_url: "https://registry.example.org".to_string(),
                    environment: "test".to_string(),
                    read_only: false,
                },
                database: DatabaseConfig {
                    url_env: "SERVICE_REGISTRY_DATABASE_URL".to_string(),
                },
                auth: AuthConfig {
                    registration_api_key_env: "SERVICE_REGISTRY_REGISTRATION_KEY".to_string(),
                },
            },
            store: crate::store::ServiceStore::from_pool(
                sqlx::PgPool::connect_lazy("postgres://invalid/local").expect("lazy pool"),
            ),
            registration_key: Some("test-key".to_string()),
        })
    }

    #[tokio::test]
    async fn service_info_matches_ga4gh_shape() {
        let response = service_info(State(test_state())).await;
        let info = response.0;
        assert_eq!(info.r#type.group, "org.ga4gh");
        assert_eq!(info.r#type.artifact, "service-registry");
        assert_eq!(info.r#type.version, "1.0.0");
        assert_eq!(info.name, "GA4GH Service Registry");
        assert!(info.id.contains("service-registry"));
    }
}
