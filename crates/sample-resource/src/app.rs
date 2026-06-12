// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::collections::HashMap;
use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use ga4gh_clearinghouse::{Clearinghouse, ClearinghouseConfig, TrustedBroker};
use reqwest::Client;
use tower_http::trace::TraceLayer;

use crate::ads::AdsClient;
use crate::config::{DatasetConfig, SampleResourceConfig};
use crate::error::SampleResourceError;
use crate::handlers;

/// Shared application state for all HTTP handlers.
pub struct AppState {
    /// Loaded service configuration.
    pub config: SampleResourceConfig,
    /// Passport and visa validator.
    pub clearinghouse: Arc<Clearinghouse>,
    /// HTTP client for upstream DUO requests.
    pub http_client: Client,
    /// Optional ADS introspection client.
    pub ads: Option<AdsClient>,
    /// Dataset catalog indexed by id.
    pub datasets: HashMap<String, DatasetConfig>,
}

impl AppState {
    /// Build application state from configuration.
    pub async fn initialize(
        config: SampleResourceConfig,
    ) -> Result<Arc<Self>, SampleResourceError> {
        let trusted_brokers = config
            .clearinghouse
            .trusted_issuers
            .iter()
            .map(|issuer| TrustedBroker::new(issuer.issuer.clone(), issuer.jwks_uri.clone()))
            .collect();
        let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
            trusted_brokers,
            config.jwks_cache_ttl(),
        ))
        .await?;

        let http_client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| SampleResourceError::Internal(format!("HTTP client: {err}")))?;

        let ads = config
            .ads
            .as_ref()
            .map(|ads_config| AdsClient::new(ads_config, http_client.clone()))
            .transpose()?;

        Ok(Arc::new(Self {
            datasets: config.datasets_by_id(),
            config,
            clearinghouse: Arc::new(clearinghouse),
            http_client,
            ads,
        }))
    }
}

impl ga4gh_clearinghouse::axum::ClearinghouseState for AppState {
    fn clearinghouse(&self) -> &Arc<Clearinghouse> {
        &self.clearinghouse
    }
}

/// Build the sample resource HTTP router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/datasets/:dataset_id", get(handlers::get_dataset))
        .route(
            "/datasets/:dataset_id/summary",
            get(handlers::get_dataset_summary),
        )
        .route("/service-info", get(handlers::service_info))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
