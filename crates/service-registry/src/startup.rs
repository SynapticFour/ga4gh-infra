// SPDX-License-Identifier: Apache-2.0

//! Server startup and operational validation.

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

use crate::app::{build_router, AppState};
use crate::config::RegistryConfig;
use crate::error::RegistryError;

/// Validate logging configuration for production safety.
pub fn validate_log_level(config: &RegistryConfig) -> Result<(), RegistryError> {
    let filter = EnvFilter::from_default_env();
    let max_level = filter.max_level_hint();
    if !config.is_development() && max_level.is_some_and(|level| level >= tracing::Level::TRACE) {
        return Err(RegistryError::Config(
            "trace logging is not permitted outside development environments".to_string(),
        ));
    }
    Ok(())
}

/// Connect to PostgreSQL, bind the HTTP server, and serve requests.
pub async fn run(config: RegistryConfig) -> anyhow::Result<()> {
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
        read_only = config.server.read_only,
        "starting GA4GH service registry"
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, DatabaseConfig, DatabaseDriver, RegistryConfig, ServerConfig};

    #[test]
    fn rejects_trace_logging_outside_development() {
        std::env::set_var("RUST_LOG", "trace");
        let config = RegistryConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8083,
                external_url: "https://registry.example.org".to_string(),
                environment: "prod".to_string(),
                read_only: true,
            },
            database: DatabaseConfig {
                driver: DatabaseDriver::Postgres,
                url: None,
                url_env: "SERVICE_REGISTRY_DATABASE_URL".to_string(),
                auto_migrate: false,
            },
            auth: AuthConfig {
                registration_api_key_env: "SERVICE_REGISTRY_REGISTRATION_KEY".to_string(),
            },
        };

        assert!(validate_log_level(&config).is_err());
        std::env::remove_var("RUST_LOG");
    }
}
