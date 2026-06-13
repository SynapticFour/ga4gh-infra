// SPDX-License-Identifier: Apache-2.0

//! All-in-one configuration and multi-service startup.

use std::path::Path;

use aai_broker::BrokerConfig;
use access_decision_service::AdsConfig;
use duo_service::DuoServiceConfig;
use mock_idp::{run as run_mock_idp, MockIdpConfig};
use serde::Deserialize;
use service_registry::RegistryConfig as ServiceRegistryConfig;
use visa_registry::RegistryConfig as VisaRegistryConfig;

use crate::africa::{apply_africa_profile, load_africa_profile, AfricaProfile};

/// Combined configuration for running all core services in one process.
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
    /// Access Decision Service settings (`access-decision-service`).
    #[serde(rename = "access_decision_service")]
    pub access_decision_service: AdsConfig,
    /// Optional Africa-mode profile (SQLite, embedded mock IdP, offline-first).
    #[serde(default)]
    pub africa: Option<AfricaProfile>,
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

    /// Parse an all-in-one configuration from a TOML string (tests and tooling).
    pub fn load_from_str(toml: &str) -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()?
            .try_deserialize()
    }
}

/// Run all core services concurrently in the current Tokio runtime.
pub async fn run_all_in_one(mut config: AllInOneConfig, africa_mode: bool) -> anyhow::Result<()> {
    if africa_mode {
        let profile = config.africa.clone().unwrap_or_else(|| AfricaProfile {
            embedded_mock_idp: true,
            offline_first: true,
            ..AfricaProfile::default()
        });
        apply_africa_profile(&mut config, &profile);
        tracing::info!("Africa-mode profile applied to all-in-one configuration");
    }

    aai_broker::validate_log_level(&config.broker).map_err(anyhow::Error::msg)?;
    visa_registry::validate_log_level(&config.visa_registry).map_err(anyhow::Error::msg)?;
    duo_service::validate_log_level(&config.duo_service).map_err(anyhow::Error::msg)?;
    service_registry::validate_log_level(&config.service_registry).map_err(anyhow::Error::msg)?;
    access_decision_service::validate_log_level(&config.access_decision_service)
        .map_err(anyhow::Error::msg)?;

    tracing::info!(
        "starting all-in-one ga4gh-infra (broker, visa-registry, duo-service, service-registry, access-decision-service)"
    );

    let embedded_mock = africa_mode
        && config
            .africa
            .as_ref()
            .is_none_or(|profile| profile.embedded_mock_idp);

    let mock_idp = if embedded_mock {
        let profile = config.africa.clone().unwrap_or_default();
        let signing_key = config
            .broker
            .signing
            .private_key_pem
            .clone()
            .replace("broker_rs256.pem", "mock_idp_rs256.pem");
        Some(tokio::spawn(async move {
            run_mock_idp(MockIdpConfig {
                issuer: format!("http://{}:{}", profile.mock_idp_host, profile.mock_idp_port),
                signing_key_pem: signing_key,
                host: profile.mock_idp_host,
                port: profile.mock_idp_port,
                ..MockIdpConfig::default()
            })
            .await
        }))
    } else {
        None
    };

    let broker = tokio::spawn(async move { aai_broker::run(config.broker).await });
    let visa_registry = tokio::spawn(async move { visa_registry::run(config.visa_registry).await });
    let duo_service = tokio::spawn(async move { duo_service::run(config.duo_service).await });
    let service_registry =
        tokio::spawn(async move { service_registry::run(config.service_registry).await });
    let access_decision_service =
        tokio::spawn(
            async move { access_decision_service::run(config.access_decision_service).await },
        );

    tokio::select! {
        result = broker => result.map_err(|err| anyhow::anyhow!("broker task panicked: {err}"))??,
        result = visa_registry => result.map_err(|err| anyhow::anyhow!("visa-registry task panicked: {err}"))??,
        result = duo_service => result.map_err(|err| anyhow::anyhow!("duo-service task panicked: {err}"))??,
        result = service_registry => result.map_err(|err| anyhow::anyhow!("service-registry task panicked: {err}"))??,
        result = access_decision_service => result.map_err(|err| anyhow::anyhow!("access-decision-service task panicked: {err}"))??,
        result = async {
            if let Some(task) = mock_idp {
                task.await.map_err(|err| anyhow::anyhow!("mock-idp task panicked: {err}"))?
            } else {
                std::future::pending::<anyhow::Result<()>>().await
            }
        } => result?,
    }

    Ok(())
}

/// Load all-in-one config and optionally apply Africa-mode from file section or CLI flag.
pub fn prepare_all_in_one_config(
    path: impl AsRef<Path>,
    africa_flag: bool,
) -> Result<(AllInOneConfig, bool), config::ConfigError> {
    let path = path.as_ref();
    let mut config = AllInOneConfig::load_from_file(path)?;
    let africa_mode = africa_flag || crate::africa::africa_mode_from_env();
    if africa_mode && config.africa.is_none() {
        config.africa = load_africa_profile(path);
    }
    Ok((config, africa_mode))
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

            [access_decision_service.server]
            host = "127.0.0.1"
            port = 8090
            external_url = "http://localhost:8090"
            environment = "development"

            [access_decision_service.database]
            url_env = "ADS_DATABASE_URL"

            [access_decision_service.oidc]
            jwks_cache_ttl_seconds = 300

            [[access_decision_service.oidc.trusted_brokers]]
            issuer = "http://localhost:8080"
            jwks_uri = "http://localhost:8080/jwks.json"

            [access_decision_service.auth]
            bootstrap_api_key_env = "ADS_DAC_API_KEY"
        "#;

        let config: AllInOneConfig = AllInOneConfig::load_from_str(toml).expect("parse config");

        assert_eq!(config.broker.server.port, 8080);
        assert_eq!(config.visa_registry.server.port, 8081);
        assert_eq!(config.duo_service.server.port, 8082);
        assert_eq!(config.service_registry.server.port, 8083);
        assert_eq!(config.access_decision_service.server.port, 8090);
    }
}
