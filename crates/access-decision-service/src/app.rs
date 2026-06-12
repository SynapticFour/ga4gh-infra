// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use ga4gh_clearinghouse::{JwksCache, TrustedBroker};
use tower_http::trace::TraceLayer;

use crate::config::AdsConfig;
use crate::error::AdsError;
use crate::handlers;
use crate::store::AdsStore;
use crate::visa_registry_client::VisaRegistryClient;

/// Shared application state for all HTTP handlers.
pub struct AppState {
    pub config: AdsConfig,
    pub store: AdsStore,
    pub jwks: Arc<JwksCache>,
    pub visa_registry: Option<VisaRegistryClient>,
}

impl AppState {
    pub async fn initialize(config: AdsConfig) -> Result<Arc<Self>, AdsError> {
        let database_url = config
            .database_url()
            .map_err(|err| AdsError::Config(format!("missing database URL: {err}")))?;
        let store = AdsStore::connect(
            &config.database,
            &database_url,
            config.webhooks.urls.clone(),
        )
        .await?;
        let jwks = Arc::new(
            JwksCache::new(
                config
                    .oidc
                    .trusted_brokers
                    .clone()
                    .into_iter()
                    .map(TrustedBroker::from)
                    .collect(),
                config.jwks_cache_ttl(),
            )
            .map_err(|err| AdsError::Config(err.to_string()))?,
        );

        if let Ok(bootstrap_key) = config.bootstrap_api_key() {
            store
                .ensure_bootstrap_api_key(&bootstrap_key, "bootstrap-dac")
                .await?;
        }

        let visa_registry = match (&config.visa_registry, config.visa_registry_api_key()) {
            (Some(cfg), Ok(Some(api_key))) => Some(VisaRegistryClient::new(cfg, api_key)?),
            (Some(_), Ok(None)) => {
                return Err(AdsError::Config(
                    "visa_registry configured but API key env var is unset".to_string(),
                ));
            }
            (Some(_), Err(err)) => {
                return Err(AdsError::Config(format!(
                    "visa_registry API key env error: {err}"
                )));
            }
            (None, _) => None,
        };

        Ok(Arc::new(Self {
            config,
            store,
            jwks,
            visa_registry,
        }))
    }
}

/// Build the ADS HTTP router under `/ads/v1`.
pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/researchers/sync", post(handlers::sync_researcher_handler))
        .route("/researchers/:id", get(handlers::get_researcher))
        .route(
            "/researchers/:id/signed-visas",
            get(handlers::get_researcher_signed_visas),
        )
        .route(
            "/researchers/:id/visas",
            get(handlers::get_researcher_visas),
        )
        .route("/datasets", post(handlers::create_dataset))
        .route("/datasets/:id", get(handlers::get_dataset))
        .route("/projects", post(handlers::create_project))
        .route("/projects/:id", get(handlers::get_project))
        .route("/duo/evaluate", post(handlers::evaluate_duo))
        .route("/access-requests", post(handlers::create_access_request))
        .route("/access-requests/:id", get(handlers::get_access_request))
        .route("/dac/requests", get(handlers::list_dac_requests))
        .route("/dac/requests/:id/approve", post(handlers::dac_approve))
        .route("/dac/requests/:id/reject", post(handlers::dac_reject))
        .route("/dac/requests/:id/escalate", post(handlers::dac_escalate))
        .route("/grants", get(handlers::list_grants))
        .route(
            "/grants/:id",
            get(handlers::get_grant).delete(handlers::revoke_grant),
        )
        .route("/introspect", post(handlers::introspect))
        .route(
            "/permission-sources",
            post(handlers::create_permission_source),
        )
        .route(
            "/permission-mappings",
            post(handlers::create_permission_mapping),
        )
        .with_state(state.clone());

    Router::new()
        .nest("/ads/v1", api)
        .route("/service-info", get(handlers::service_info))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
