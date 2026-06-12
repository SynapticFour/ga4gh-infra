// SPDX-License-Identifier: Apache-2.0

//! All-in-one configuration and multi-service startup.

use std::path::Path;

use aai_broker::BrokerConfig;
use duo_service::DuoServiceConfig;
use serde::Deserialize;
use service_registry::RegistryConfig as ServiceRegistryConfig;
use visa_registry::RegistryConfig as VisaRegistryConfig;

/// Combined configuration for running all core services in one process.
///
/// Each top-level section maps to the corresponding service's existing config struct.
/// SQLite defaults for desktop use are documented in Section B; until then use PostgreSQL
/// URLs via the environment variables referenced in each nested section.
#[derive(Debug, Clone, Deserialize)]
pub struct AllInOneConfig {
    /// AAI broker settings (`aai-broker`).
    pub broker: BrokerConfig,
    /// Visa registry settings (`visa-registry`).
    #[serde(rename = "visa_registry")]
    pub visa_registry: VisaRegistryConfig,
    /// DUO service settings (`duo-service`).
    #[serde(rename = "duo_service")]
    pub duo_service: DuoServiceConfig,
    /// Service registry settings (`service-registry`).
    #[serde(rename = "service_registry")]
    pub service_registry: ServiceRegistryConfig,
}

impl AllInOneConfig {
    /// Load an all-in-one configuration file from disk.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        config::Config::builder()
            .add_source(config::File::from(path))
            .build()?
            .try_deserialize()
    }
}

/// Run all four services concurrently in the current Tokio runtime.
pub async fn run_all_in_one(config: AllInOneConfig) -> anyhow::Result<()> {
    aai_broker::validate_log_level(&config.broker).map_err(anyhow::Error::msg)?;
    visa_registry::validate_log_level(&config.visa_registry).map_err(anyhow::Error::msg)?;
    duo_service::validate_log_level(&config.duo_service).map_err(anyhow::Error::msg)?;
    service_registry::validate_log_level(&config.service_registry).map_err(anyhow::Error::msg)?;

    tracing::info!("starting all-in-one ga4gh-infra (broker, visa-registry, duo-service, service-registry)");

    let broker = tokio::spawn(async move { aai_broker::run(config.broker).await });
    let visa_registry = tokio::spawn(async move { visa_registry::run(config.visa_registry).await });
    let duo_service = tokio::spawn(async move { duo_service::run(config.duo_service).await });
    let service_registry =
        tokio::spawn(async move { service_registry::run(config.service_registry).await });

    tokio::select! {
        result = broker => result.map_err(|err| anyhow::anyhow!("broker task panicked: {err}"))??,
        result = visa_registry => result.map_err(|err| anyhow::anyhow!("visa-registry task panicked: {err}"))??,
        result = duo_service => result.map_err(|err| anyhow::anyhow!("duo-service task panicked: {err}"))??,
        result = service_registry => result.map_err(|err| anyhow::anyhow!("service-registry task panicked: {err}"))??,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_in_one_config_shape() {
        let toml = r#"
            [broker.server]
            host = "127.0.0.1"
            port = 8080
            external_url = "http://localhost:8080"
            environment = "development"

            [broker.signing]
            private_key_pem = "/secrets/broker.pem"
            passport_lifetime_seconds = 3600
            token_lifetime_seconds = 3600

            [broker.session]
            cookie_secret_env = "BROKER_COOKIE_SECRET"
            session_lifetime_seconds = 600

            [[broker.upstream_idps]]
            name = "mock"
            issuer = "http://localhost:9000"
            client_id = "ga4gh-broker"
            client_secret_env = "MOCK_IDP_CLIENT_SECRET"
            scopes = ["openid"]

            [[broker.visa_sources]]
            name = "local"
            url = "http://127.0.0.1:8081"

            [visa_registry.server]
            host = "127.0.0.1"
            port = 8081
            external_url = "http://localhost:8081"
            environment = "development"

            [visa_registry.signing]
            private_key_pem = "/secrets/registry.pem"
            visa_lifetime_seconds = 86400

            [visa_registry.database]
            url_env = "REGISTRY_DATABASE_URL"

            [visa_registry.auth]
            bootstrap_api_key_env = "REGISTRY_BOOTSTRAP_API_KEY"

            [duo_service.server]
            host = "127.0.0.1"
            port = 8082
            external_url = "http://localhost:8082"
            environment = "development"

            [service_registry.server]
            host = "127.0.0.1"
            port = 8083
            external_url = "http://localhost:8083"
            environment = "development"
            read_only = false

            [service_registry.database]
            url_env = "SERVICE_REGISTRY_DATABASE_URL"

            [service_registry.auth]
            registration_api_key_env = "SERVICE_REGISTRY_REGISTRATION_KEY"
        "#;

        let config: AllInOneConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");

        assert_eq!(config.broker.server.port, 8080);
        assert_eq!(config.visa_registry.server.port, 8081);
        assert_eq!(config.duo_service.server.port, 8082);
        assert_eq!(config.service_registry.server.port, 8083);
    }
}
