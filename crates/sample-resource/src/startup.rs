// SPDX-License-Identifier: Apache-2.0

//! Server startup and operational validation.

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

use crate::app::{build_router, AppState};
use crate::config::SampleResourceConfig;
use crate::error::SampleResourceError;

/// Validate logging configuration for production safety.
pub fn validate_log_level(config: &SampleResourceConfig) -> Result<(), SampleResourceError> {
    let filter = EnvFilter::from_default_env();
    let max_level = filter.max_level_hint();
    if !config.is_development() && max_level.is_some_and(|level| level >= tracing::Level::TRACE) {
        return Err(SampleResourceError::Config(
            "trace logging is not permitted outside development environments".to_string(),
        ));
    }
    Ok(())
}

/// Initialize the clearinghouse, bind the HTTP server, and serve requests.
pub async fn run(config: SampleResourceConfig) -> anyhow::Result<()> {
    let state = AppState::initialize(config.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    let app = build_router(state);
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|err| anyhow::anyhow!("invalid listen address: {err}"))?;

    tracing::info!(
        %addr,
        external_url = %config.external_url(),
        datasets = config.datasets.len(),
        "starting sample resource service"
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ClearinghouseSection, DuoServiceSection, ServerConfig};

    #[test]
    fn rejects_trace_logging_outside_development() {
        std::env::set_var("RUST_LOG", "trace");
        let config = SampleResourceConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8084,
                external_url: "http://localhost:8084".to_string(),
                environment: "prod".to_string(),
            },
            clearinghouse: ClearinghouseSection {
                jwks_cache_ttl_seconds: 300,
                trusted_issuers: Vec::new(),
            },
            duo_service: DuoServiceSection {
                url: "http://localhost:8082".to_string(),
            },
            ads: None,
            datasets: Vec::new(),
        };

        assert!(validate_log_level(&config).is_err());
        std::env::remove_var("RUST_LOG");
    }
}
