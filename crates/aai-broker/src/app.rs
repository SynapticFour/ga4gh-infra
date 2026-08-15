// SPDX-License-Identifier: Apache-2.0

//! Application state and HTTP router construction.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::ads::AdsClient;
use crate::config::BrokerConfig;
use crate::handlers;
use crate::keys::SigningKeys;
use crate::passport_ledger::PassportLedger;
use crate::profile::ProfileStore;
use crate::session::SessionManager;
use crate::upstream::{build_http_client, UpstreamRegistry};
use crate::visas::VisaSourceClient;
use ga4gh_http::SlidingWindowLimiter;
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
    /// Sliding-window limiter for `/login` and `/callback`.
    pub login_limiter: SlidingWindowLimiter,
    /// Issued and revoked Passport JTIs.
    pub passport_ledger: PassportLedger,
}

impl AppState {
    /// Build application state from configuration, discovering upstream IdPs on startup.
    pub async fn initialize(config: BrokerConfig) -> Result<Arc<Self>, crate::error::BrokerError> {
        let cookie_secret = config.cookie_secret().map_err(|err| {
            crate::error::BrokerError::Config(format!("missing cookie secret: {err}"))
        })?;
        config
            .reject_insecure_bootstrap_secrets()
            .map_err(crate::error::BrokerError::Config)?;
        let keys = SigningKeys::from_pem_file(&config.signing.private_key_pem)?;
        let mut keys = keys;
        keys.merge_previous_pems(&config.signing.previous_key_pems)?;
        let http_client = build_http_client()?;
        let upstream = UpstreamRegistry::discover_all(&config, &http_client).await?;
        let visa_sources = {
            let mut sources = Vec::new();
            for source in &config.visa_sources {
                match VisaSourceClient::new(source) {
                    Ok(client) => sources.push(client),
                    Err(err) if source.required => return Err(err),
                    Err(err) => tracing::warn!(
                        source = %source.name,
                        error = %err,
                        "skipping optional visa source"
                    ),
                }
            }
            sources
        };
        let ads = config.ads.as_ref().map(AdsClient::new).transpose()?;
        let login_limiter = SlidingWindowLimiter::new(
            config.server.login_rate_limit_per_minute,
            Duration::from_secs(60),
        );
        let passport_ledger = PassportLedger::new(
            config
                .server
                .passport_ledger_path
                .as_ref()
                .map(std::path::PathBuf::from),
        );

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
            login_limiter,
            passport_ledger,
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
        .route("/revoked-passports", get(handlers::list_revoked_passports))
        .route("/revoke-passports", post(handlers::revoke_passports))
        .route("/userinfo", get(handlers::userinfo))
        .route("/service-info", get(handlers::service_info))
        .route("/health", get(ga4gh_http::health))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn(ga4gh_http::security_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
