// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::config::DuoServiceConfig;
use crate::error::DuoServiceError;
use crate::handlers;
use crate::terms::DuoCatalog;

/// Shared application state for all HTTP handlers.
pub struct AppState {
    /// Loaded service configuration.
    pub config: DuoServiceConfig,
    /// Compiled DUO term catalog.
    pub catalog: DuoCatalog,
}

impl AppState {
    /// Build application state from configuration.
    pub fn initialize(config: DuoServiceConfig) -> Result<Arc<Self>, DuoServiceError> {
        Ok(Arc::new(Self {
            catalog: DuoCatalog::from_embedded()?,
            config,
        }))
    }
}

/// Build the DUO service HTTP router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/terms", get(handlers::list_terms))
        .route("/terms/:code", get(handlers::get_term))
        .route("/match", post(handlers::match_duo))
        .route("/service-info", get(handlers::service_info))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
