// SPDX-License-Identifier: Apache-2.0

//! Server startup and operational validation.

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

use crate::app::{build_router, AppState};
use crate::config::BrokerConfig;
use crate::error::BrokerError;

/// Validate logging configuration for production safety.
pub fn validate_log_level(config: &BrokerConfig) -> Result<(), BrokerError> {
    let filter = EnvFilter::from_default_env();
    let max_level = filter.max_level_hint();
    if !config.is_development() && max_level.is_some_and(|level| level >= tracing::Level::TRACE) {
        return Err(BrokerError::Config(
            "trace logging is not permitted outside development environments".to_string(),
        ));
    }
    Ok(())
}

/// Discover upstream IdPs, bind the HTTP server, and serve requests.
pub async fn run(config: BrokerConfig) -> anyhow::Result<()> {
    let state = AppState::initialize(config.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    let app = build_router(state);
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|err| anyhow::anyhow!("invalid listen address: {err}"))?;

    tracing::info!(%addr, issuer = %config.issuer_url(), "starting GA4GH AAI broker");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ServerConfig, SessionConfig, SigningConfig};

    #[test]
    fn rejects_trace_logging_outside_development() {
        std::env::set_var("RUST_LOG", "trace");
        let config = BrokerConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                external_url: "https://broker.example.org".to_string(),
                environment: "prod".to_string(),
            },
            signing: SigningConfig {
                private_key_pem: "/tmp/key.pem".to_string(),
                passport_lifetime_seconds: 3600,
                token_lifetime_seconds: 3600,
            },
            session: SessionConfig {
                cookie_secret_env: "BROKER_COOKIE_SECRET".to_string(),
                session_lifetime_seconds: 600,
            },
            upstream_idps: vec![],
            visa_sources: vec![],
            ads: None,
        };

        assert!(validate_log_level(&config).is_err());
        std::env::remove_var("RUST_LOG");
    }
}
