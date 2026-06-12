// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::config::RegistryConfig;
use crate::error::RegistryError;
use crate::handlers;
use crate::store::ServiceStore;

/// Shared application state for all HTTP handlers.
pub struct AppState {
    /// Loaded registry configuration.
    pub config: RegistryConfig,
    /// PostgreSQL-backed service store.
    pub store: ServiceStore,
    /// Shared secret for internal service registration, when configured.
    pub registration_key: Option<String>,
}

impl AppState {
    /// Build application state from configuration.
    pub async fn initialize(config: RegistryConfig) -> Result<Arc<Self>, RegistryError> {
        let database_url = config
            .database_url()
            .map_err(|err| RegistryError::Config(format!("missing database URL: {err}")))?;
        let store = ServiceStore::connect(&database_url).await?;
        let registration_key = config.registration_api_key().ok();

        if !config.server.read_only && registration_key.is_none() {
            return Err(RegistryError::Config(
                "registration API key is required when registry is not read-only".to_string(),
            ));
        }

        Ok(Arc::new(Self {
            config,
            store,
            registration_key,
        }))
    }
}

/// Build the service registry HTTP router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/services",
            get(handlers::list_services).post(handlers::register_service),
        )
        .route("/services/types", get(handlers::list_service_types))
        .route(
            "/services/:serviceId",
            get(handlers::get_service).delete(handlers::delete_service),
        )
        .route("/service-info", get(handlers::service_info))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
