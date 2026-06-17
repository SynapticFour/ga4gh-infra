// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

use crate::config::AgreementRegistryConfig;
use crate::demo_profiles::register_demo_profiles;
use crate::handlers;
use crate::http_error::AgreementRegistryHttpError;
use crate::registry::InMemoryRegistry;

/// Shared application state.
pub struct AppState {
    pub config: AgreementRegistryConfig,
    pub registry: Arc<RwLock<InMemoryRegistry>>,
}

impl AppState {
    pub fn initialize(
        config: AgreementRegistryConfig,
    ) -> Result<Arc<Self>, AgreementRegistryHttpError> {
        let mut registry = InMemoryRegistry::new()
            .with_seed_templates()
            .map_err(AgreementRegistryHttpError::Registry)?;
        register_demo_profiles(&mut registry);
        Ok(Arc::new(Self {
            config,
            registry: Arc::new(RwLock::new(registry)),
        }))
    }
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/service-info", get(handlers::service_info))
        .route("/health", get(handlers::health))
        .route("/templates", get(handlers::list_templates))
        .route("/templates/:id", get(handlers::get_template))
        .route("/profiles", get(handlers::list_profiles).post(handlers::register_profile))
        .route("/profiles/:id", get(handlers::get_profile))
        .route("/compatibility-check", post(handlers::compatibility_check))
        .route("/decisions", get(handlers::list_decisions))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
