// SPDX-License-Identifier: Apache-2.0

//! Sample resource service configuration loaded from TOML and environment variables.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

/// Top-level sample resource service configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SampleResourceConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
    /// Clearinghouse trust and cache settings.
    pub clearinghouse: ClearinghouseSection,
    /// Optional DUO service used for summary endpoints.
    pub duo_service: DuoServiceSection,
    /// Optional ADS introspection for grant-based access checks.
    #[serde(default)]
    pub ads: Option<AdsSection>,
    /// Protected datasets exposed by this service.
    pub datasets: Vec<DatasetConfig>,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind address host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Public base URL of this service (no trailing slash).
    pub external_url: String,
    /// Deployment environment label (`prod`, `test`, `dev`, `staging`, `development`).
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "dev".to_string()
}

/// Clearinghouse configuration section.
#[derive(Debug, Clone, Deserialize)]
pub struct ClearinghouseSection {
    /// JWKS cache TTL in seconds.
    pub jwks_cache_ttl_seconds: u64,
    /// Trusted Passport brokers and Visa issuers.
    pub trusted_issuers: Vec<TrustedIssuerConfig>,
}

/// Trusted issuer entry in configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TrustedIssuerConfig {
    /// Expected JWT `iss` claim value.
    pub issuer: String,
    /// JWKS URL used to resolve signing keys for this issuer.
    pub jwks_uri: String,
}

/// DUO service integration settings.
#[derive(Debug, Clone, Deserialize)]
pub struct DuoServiceSection {
    /// Base URL of the DUO matching service.
    pub url: String,
}

/// ADS introspection integration settings.
#[derive(Debug, Clone, Deserialize)]
pub struct AdsSection {
    /// Base URL of the Access Decision Service (no trailing slash).
    pub url: String,
    /// Environment variable holding the DAC/service API key for introspection.
    #[serde(default = "default_ads_api_key_env")]
    pub api_key_env: String,
}

fn default_ads_api_key_env() -> String {
    "ADS_DAC_API_KEY".to_string()
}

/// Dataset metadata and access policy inputs.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DatasetConfig {
    /// Stable dataset identifier referenced by controlled-access visas.
    pub id: String,
    /// Human-readable dataset name.
    pub name: String,
    /// Optional description returned to authorized callers.
    #[serde(default)]
    pub description: Option<String>,
    /// DUO codes attached to the dataset for summary endpoints.
    #[serde(default)]
    pub duo: Vec<String>,
    /// Default intended-use DUO codes when callers omit `X-GA4GH-Intended-Use`.
    #[serde(default)]
    pub default_intended_use: Vec<String>,
}

impl SampleResourceConfig {
    /// Load configuration from a TOML file, with optional `SAMPLE_RESOURCE__` overrides.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("SAMPLE_RESOURCE").separator("__"))
            .build()?;
        settings.try_deserialize()
    }

    /// Public base URL for service metadata.
    pub fn external_url(&self) -> &str {
        self.server.external_url.trim_end_matches('/')
    }

    /// JWKS cache TTL for the clearinghouse.
    pub fn jwks_cache_ttl(&self) -> Duration {
        Duration::from_secs(self.clearinghouse.jwks_cache_ttl_seconds)
    }

    /// Index datasets by id for handler lookup.
    pub fn datasets_by_id(&self) -> HashMap<String, DatasetConfig> {
        self.datasets
            .iter()
            .map(|dataset| (dataset.id.clone(), dataset.clone()))
            .collect()
    }

    /// Returns `true` when the deployment is explicitly marked as development.
    pub fn is_development(&self) -> bool {
        matches!(
            self.server.environment.as_str(),
            "development" | "dev" | "local"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example_config_shape() {
        let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 8084
            external_url = "http://localhost:8084"
            environment = "development"

            [clearinghouse]
            jwks_cache_ttl_seconds = 300

            [[clearinghouse.trusted_issuers]]
            issuer = "http://localhost:8080"
            jwks_uri = "http://aai-broker:8080/jwks.json"

            [duo_service]
            url = "http://duo-service:8082"

            [[datasets]]
            id = "dataset-registered-access-demo"
            name = "Registered Access Demo Cohort"
            duo = ["GRU", "NPU"]
            default_intended_use = ["HMB", "NPU"]
        "#;

        let config: SampleResourceConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");

        assert_eq!(config.server.port, 8084);
        assert_eq!(config.datasets.len(), 1);
        assert_eq!(
            config.datasets_by_id()["dataset-registered-access-demo"].duo,
            vec!["GRU", "NPU"]
        );
    }
}
