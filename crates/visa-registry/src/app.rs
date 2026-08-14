// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::collections::HashMap;
use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::config::RegistryConfig;
use crate::error::RegistryError;
use crate::handlers;
use crate::keys::SigningKeys;
use crate::store::VisaStore;

/// Shared application state for all HTTP handlers.
pub struct AppState {
    /// Loaded registry configuration.
    pub config: RegistryConfig,
    /// Visa JWT signing keys and JWKS material.
    pub keys: SigningKeys,
    /// Database-backed assertion store.
    pub store: VisaStore,
    /// Signed visa JWTs keyed by assertion id (invalidated on revoke).
    pub visa_jwts: RwLock<HashMap<Uuid, String>>,
}

impl AppState {
    /// Build application state from configuration.
    pub async fn initialize(config: RegistryConfig) -> Result<Arc<Self>, RegistryError> {
        let database_url = config
            .database_url()
            .map_err(|err| RegistryError::Config(format!("missing database URL: {err}")))?;
        let store = VisaStore::connect(&config.database, &database_url).await?;
        let keys = SigningKeys::from_pem_file(&config.signing.private_key_pem)?;

        if let Ok(bootstrap_key) = config.bootstrap_api_key() {
            store
                .ensure_bootstrap_api_key(&bootstrap_key, "bootstrap")
                .await?;
        }

        Ok(Arc::new(Self {
            config,
            keys,
            store,
            visa_jwts: RwLock::new(HashMap::new()),
        }))
    }
}

/// Build the visa registry HTTP router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/visas",
            post(handlers::create_visa).get(handlers::list_visas),
        )
        .route("/visas/:id", delete(handlers::delete_visa))
        .route("/jwks.json", get(handlers::jwks))
        .route("/service-info", get(handlers::service_info))
        .route("/health", get(handlers::health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
