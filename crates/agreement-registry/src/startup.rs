// SPDX-License-Identifier: Apache-2.0

//! Server startup.

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

use crate::app::{build_router, AppState};
use crate::config::AgreementRegistryConfig;
use crate::http_error::AgreementRegistryHttpError;

pub fn validate_log_level(
    config: &AgreementRegistryConfig,
) -> Result<(), AgreementRegistryHttpError> {
    let filter = EnvFilter::from_default_env();
    let max_level = filter.max_level_hint();
    if !config.is_development() && max_level.is_some_and(|level| level >= tracing::Level::TRACE) {
        return Err(AgreementRegistryHttpError::Config(
            "trace logging is not permitted outside development environments".to_string(),
        ));
    }
    Ok(())
}

pub async fn run(config: AgreementRegistryConfig) -> anyhow::Result<()> {
    let state = AppState::initialize(config.clone()).map_err(anyhow::Error::msg)?;
    let app = build_router(state);
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|err| anyhow::anyhow!("invalid listen address: {err}"))?;

    tracing::info!(
        %addr,
        external_url = %config.external_url(),
        "starting agreement registry"
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
