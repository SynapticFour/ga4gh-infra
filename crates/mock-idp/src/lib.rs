// SPDX-License-Identifier: Apache-2.0

//! Minimal OIDC provider for docker-compose, end-to-end tests, and embedded Africa-mode.

pub mod idp;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

/// Runtime configuration for the mock OIDC IdP.
#[derive(Debug, Clone)]
pub struct MockIdpConfig {
    /// Issuer URL advertised in discovery and ID tokens (no trailing slash).
    pub issuer: String,
    /// Browser-facing base URL for the authorization endpoint (defaults to `issuer`).
    pub public_base_url: Option<String>,
    /// Path to PEM-encoded RS256 private key.
    pub signing_key_pem: String,
    /// Bind host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Subject claim for issued ID tokens.
    pub subject: String,
    /// OAuth client id accepted by the token endpoint.
    pub client_id: String,
    /// OAuth client secret accepted by the token endpoint.
    pub client_secret: String,
}

impl Default for MockIdpConfig {
    fn default() -> Self {
        Self {
            issuer: "http://127.0.0.1:9000".to_string(),
            public_base_url: None,
            signing_key_pem: "/secrets/mock_idp_rs256.pem".to_string(),
            host: "127.0.0.1".to_string(),
            port: 9000,
            subject: "researcher@uni-heidelberg.de".to_string(),
            client_id: "ga4gh-broker".to_string(),
            client_secret: "mock-client-secret".to_string(),
        }
    }
}

impl MockIdpConfig {
    /// Load configuration from environment variables (same names as the standalone binary).
    pub fn from_env() -> Self {
        Self {
            issuer: std::env::var("MOCK_IDP_ISSUER")
                .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string()),
            public_base_url: std::env::var("MOCK_IDP_PUBLIC_URL").ok(),
            signing_key_pem: std::env::var("MOCK_IDP_SIGNING_KEY_PEM")
                .unwrap_or_else(|_| "/secrets/mock_idp_rs256.pem".to_string()),
            host: std::env::var("MOCK_IDP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("MOCK_IDP_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(9000),
            subject: std::env::var("MOCK_IDP_SUBJECT")
                .unwrap_or_else(|_| "researcher@uni-heidelberg.de".to_string()),
            client_id: std::env::var("MOCK_IDP_CLIENT_ID")
                .unwrap_or_else(|_| "ga4gh-broker".to_string()),
            client_secret: std::env::var("MOCK_IDP_CLIENT_SECRET")
                .unwrap_or_else(|_| "mock-client-secret".to_string()),
        }
    }
}

/// Build the mock IdP HTTP router.
pub fn build_router(config: &MockIdpConfig) -> anyhow::Result<Router> {
    std::env::set_var("MOCK_IDP_CLIENT_ID", &config.client_id);
    std::env::set_var("MOCK_IDP_CLIENT_SECRET", &config.client_secret);

    let issuer = config.issuer.trim_end_matches('/');
    let public_base_url = config
        .public_base_url
        .as_deref()
        .unwrap_or(issuer)
        .trim_end_matches('/');
    let state = Arc::new(idp::MockIdpState::new(
        issuer,
        public_base_url,
        &config.signing_key_pem,
        config.subject.clone(),
    )?);

    Ok(Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(idp::openid_configuration),
        )
        .route("/jwks.json", get(idp::jwks))
        .route("/oauth/authorize", get(idp::authorize))
        .route("/oauth/token", post(idp::token))
        .route("/oauth/userinfo", get(idp::userinfo))
        .with_state(state))
}

/// Run the mock IdP until the listener is closed or the server exits.
pub async fn run(config: MockIdpConfig) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let app = build_router(&config)?;
    tracing::info!(%addr, issuer = %config.issuer, "starting mock OIDC IdP");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
