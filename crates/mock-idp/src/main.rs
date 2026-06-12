// SPDX-License-Identifier: Apache-2.0

//! Minimal OIDC provider for docker-compose and end-to-end tests.

mod idp;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let issuer =
        std::env::var("MOCK_IDP_ISSUER").unwrap_or_else(|_| "http://mock-idp:9000".to_string());
    let key_path = std::env::var("MOCK_IDP_SIGNING_KEY_PEM")
        .unwrap_or_else(|_| "/secrets/mock_idp_rs256.pem".to_string());
    let host = std::env::var("MOCK_IDP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("MOCK_IDP_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(9000);
    let subject = std::env::var("MOCK_IDP_SUBJECT")
        .unwrap_or_else(|_| "researcher@uni-heidelberg.de".to_string());

    let state = Arc::new(idp::MockIdpState::new(
        issuer.trim_end_matches('/'),
        &key_path,
        subject,
    )?);

    let app = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(idp::openid_configuration),
        )
        .route("/jwks.json", get(idp::jwks))
        .route("/oauth/authorize", get(idp::authorize))
        .route("/oauth/token", post(idp::token))
        .route("/oauth/userinfo", get(idp::userinfo))
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    tracing::info!(%addr, issuer = %issuer, "starting mock OIDC IdP");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
