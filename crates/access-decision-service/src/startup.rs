// SPDX-License-Identifier: Apache-2.0

//! HTTP server startup.

use std::net::SocketAddr;

use axum::serve;
use tokio::net::TcpListener;
use tracing::info;

use crate::app::{build_router, AppState};
use crate::config::AdsConfig;
use crate::error::AdsError;

/// Validate configured log level (placeholder for parity with other services).
pub fn validate_log_level(_config: &AdsConfig) -> Result<(), String> {
    Ok(())
}

/// Bind and serve the ADS HTTP API.
pub async fn run(config: AdsConfig) -> anyhow::Result<()> {
    let state = AppState::initialize(config.clone())
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let router = build_router(state);
    let addr = SocketAddr::new(
        config.server.host.parse().map_err(|err| {
            anyhow::anyhow!("invalid server.host `{}`: {err}", config.server.host)
        })?,
        config.server.port,
    );

    info!(%addr, "starting access-decision-service");
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| AdsError::Internal(format!("bind {addr}: {err}")))?;
    serve(listener, router)
        .await
        .map_err(|err| anyhow::anyhow!("server error: {err}"))
}
