// SPDX-License-Identifier: Apache-2.0

//! ADS configuration loaded from TOML and environment variables.

use std::path::Path;
use std::time::Duration;

use ga4gh_clearinghouse::TrustedBroker;
use serde::Deserialize;

/// Top-level ADS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AdsConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
    /// Database connection settings.
    pub database: DatabaseConfig,
    /// OAuth2/OIDC JWT validation settings.
    pub oidc: OidcConfig,
    /// DAC and admin API key authentication.
    pub auth: AuthConfig,
    /// Visa export settings for AAI integration.
    #[serde(default)]
    pub visas: VisaExportConfig,
    /// Optional visa-registry used to sign exported visa claims.
    #[serde(default)]
    pub visa_registry: Option<VisaRegistryConfig>,
    /// Webhook URLs notified on audit events.
    #[serde(default)]
    pub webhooks: WebhookConfig,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Public base URL (no trailing slash).
    pub external_url: String,
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "dev".to_string()
}

/// Supported database backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    #[default]
    Postgres,
    Sqlite,
}

/// Database persistence configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub driver: DatabaseDriver,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default = "default_url_env")]
    pub url_env: String,
    #[serde(default)]
    pub auto_migrate: bool,
}

fn default_url_env() -> String {
    "ADS_DATABASE_URL".to_string()
}

/// OIDC broker trust for researcher-facing endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    /// Trusted GA4GH AAI broker issuers.
    pub trusted_brokers: Vec<TrustedBrokerConfig>,
    /// JWKS cache TTL in seconds.
    #[serde(default = "default_jwks_ttl")]
    pub jwks_cache_ttl_seconds: u64,
}

/// Trusted broker entry in ADS configuration files.
#[derive(Debug, Clone, Deserialize)]
pub struct TrustedBrokerConfig {
    /// Expected JWT `iss` claim value.
    pub issuer: String,
    /// JWKS URL used to resolve signing keys.
    pub jwks_uri: String,
}

impl From<TrustedBrokerConfig> for TrustedBroker {
    fn from(value: TrustedBrokerConfig) -> Self {
        TrustedBroker::new(value.issuer, value.jwks_uri)
    }
}

fn default_jwks_ttl() -> u64 {
    300
}

/// DAC/admin API key settings.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Environment variable for bootstrap DAC API key (hashed at startup).
    #[serde(default = "default_api_key_env")]
    pub bootstrap_api_key_env: String,
}

fn default_api_key_env() -> String {
    "ADS_DAC_API_KEY".to_string()
}

/// Visa export configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct VisaExportConfig {
    /// Organization URL used as visa `source` for ADS-issued visas.
    #[serde(default = "default_visa_source")]
    pub default_source_url: String,
}

impl Default for VisaExportConfig {
    fn default() -> Self {
        Self {
            default_source_url: default_visa_source(),
        }
    }
}

fn default_visa_source() -> String {
    "https://ads.example.org".to_string()
}

/// Visa registry signing integration.
#[derive(Debug, Clone, Deserialize)]
pub struct VisaRegistryConfig {
    /// Base URL of the visa registry (no trailing slash).
    pub url: String,
    /// Environment variable holding the DAC API key for POST /visas.
    #[serde(default = "default_vr_api_key_env")]
    pub api_key_env: String,
}

fn default_vr_api_key_env() -> String {
    "REGISTRY_BOOTSTRAP_API_KEY".to_string()
}

/// Webhook notification settings.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WebhookConfig {
    /// HTTP endpoints that receive POSTed audit events.
    #[serde(default)]
    pub urls: Vec<String>,
}

impl AdsConfig {
    /// Load configuration from a TOML file.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .build()?
            .try_deserialize()
    }

    /// Public external URL without trailing slash.
    pub fn external_url(&self) -> &str {
        self.server.external_url.trim_end_matches('/')
    }

    /// Resolved database connection URL.
    pub fn database_url(&self) -> Result<String, std::env::VarError> {
        if let Some(url) = &self.database.url {
            return Ok(url.clone());
        }
        std::env::var(&self.database.url_env)
    }

    /// Bootstrap DAC API key from environment.
    pub fn bootstrap_api_key(&self) -> Result<String, std::env::VarError> {
        std::env::var(&self.auth.bootstrap_api_key_env)
    }

    /// Visa registry API key when signing integration is configured.
    pub fn visa_registry_api_key(&self) -> Result<Option<String>, std::env::VarError> {
        match &self.visa_registry {
            Some(cfg) => std::env::var(&cfg.api_key_env).map(Some),
            None => Ok(None),
        }
    }

    /// JWKS cache TTL as a duration.
    pub fn jwks_cache_ttl(&self) -> Duration {
        Duration::from_secs(self.oidc.jwks_cache_ttl_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_external_url() {
        let cfg = AdsConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8090,
                external_url: "https://ads.example.org/".to_string(),
                environment: "test".to_string(),
            },
            database: DatabaseConfig {
                driver: DatabaseDriver::Sqlite,
                url: Some("sqlite::memory:".to_string()),
                url_env: "ADS_DATABASE_URL".to_string(),
                auto_migrate: true,
            },
            oidc: OidcConfig {
                trusted_brokers: vec![TrustedBrokerConfig {
                    issuer: "https://broker.example.org".to_string(),
                    jwks_uri: "https://broker.example.org/jwks.json".to_string(),
                }],
                jwks_cache_ttl_seconds: 300,
            },
            auth: AuthConfig {
                bootstrap_api_key_env: "ADS_DAC_API_KEY".to_string(),
            },
            visas: VisaExportConfig::default(),
            visa_registry: None,
            webhooks: WebhookConfig::default(),
        };
        assert_eq!(cfg.external_url(), "https://ads.example.org");
    }
}
