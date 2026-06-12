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

/// Connect to the configured database, bind the HTTP server, and serve requests.
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
        issuer = %config.issuer_url(),
        "starting GA4GH visa registry"
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, DatabaseConfig, ServerConfig, SigningConfig};

    #[test]
    fn rejects_trace_logging_outside_development() {
        std::env::set_var("RUST_LOG", "trace");
        let config = RegistryConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8081,
                external_url: "https://visas.example.org".to_string(),
                environment: "prod".to_string(),
            },
            signing: SigningConfig {
                private_key_pem: "/tmp/key.pem".to_string(),
                visa_lifetime_seconds: 86400,
            },
            database: DatabaseConfig {
                driver: crate::config::DatabaseDriver::Postgres,
                url: None,
                url_env: "REGISTRY_DATABASE_URL".to_string(),
                auto_migrate: false,
            },
            auth: AuthConfig {
                bootstrap_api_key_env: "REGISTRY_BOOTSTRAP_API_KEY".to_string(),
            },
        };

        assert!(validate_log_level(&config).is_err());
        std::env::remove_var("RUST_LOG");
    }
}
