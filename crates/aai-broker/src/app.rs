// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::ads::AdsClient;
use crate::config::BrokerConfig;
use crate::handlers;
use crate::keys::SigningKeys;
use crate::profile::ProfileStore;
use crate::session::SessionManager;
use crate::upstream::{build_http_client, UpstreamRegistry};
use crate::visas::VisaSourceClient;
use reqwest::Client;

/// Shared application state for all HTTP handlers.
pub struct AppState {
    /// Loaded broker configuration.
    pub config: BrokerConfig,
    /// Passport signing keys and JWKS material.
    pub keys: SigningKeys,
    /// RP login session cookie manager.
    pub sessions: SessionManager,
    /// Discovered upstream IdP clients.
    pub upstream: UpstreamRegistry,
    /// Visa source HTTP clients.
    pub visa_sources: Vec<VisaSourceClient>,
    /// Optional ADS client for researcher sync and signed visas.
    pub ads: Option<AdsClient>,
    /// Cached researcher profiles for `/userinfo`.
    pub profiles: ProfileStore,
    /// Shared HTTP client for upstream OIDC requests.
    pub http_client: Client,
}

impl AppState {
    /// Build application state from configuration, discovering upstream IdPs on startup.
    pub async fn initialize(config: BrokerConfig) -> Result<Arc<Self>, crate::error::BrokerError> {
        let cookie_secret = config.cookie_secret().map_err(|err| {
            crate::error::BrokerError::Config(format!("missing cookie secret: {err}"))
        })?;
        let keys = SigningKeys::from_pem_file(&config.signing.private_key_pem)?;
        let http_client = build_http_client()?;
        let upstream = UpstreamRegistry::discover_all(&config, &http_client).await?;
        let visa_sources = config
            .visa_sources
            .iter()
            .map(VisaSourceClient::new)
            .collect::<Result<Vec<_>, _>>()?;
        let ads = config.ads.as_ref().map(AdsClient::new).transpose()?;

        Ok(Arc::new(Self {
            sessions: SessionManager::new(
                &cookie_secret,
                config.session.session_lifetime_seconds,
                config.secure_cookies(),
            ),
            config,
            keys,
            upstream,
            visa_sources,
            ads,
            profiles: ProfileStore::default(),
            http_client,
        }))
    }
}

/// Build the broker HTTP router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/login", get(handlers::login_default))
        .route("/login/:idp_name", get(handlers::login_named))
        .route("/callback", get(handlers::callback))
        .route(
            "/.well-known/openid-configuration",
            get(handlers::openid_configuration),
        )
        .route("/jwks.json", get(handlers::jwks))
        .route("/userinfo", get(handlers::userinfo))
        .route("/service-info", get(handlers::service_info))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
