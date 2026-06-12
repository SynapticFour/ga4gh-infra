// SPDX-License-Identifier: Apache-2.0

//! Broker configuration loaded from TOML and environment variables.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Top-level broker configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BrokerConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
    /// Passport and access-token signing settings.
    pub signing: SigningConfig,
    /// Short-lived RP session cookie settings.
    pub session: SessionConfig,
    /// Upstream OIDC identity providers.
    pub upstream_idps: Vec<UpstreamIdpConfig>,
    /// Visa assertion sources queried during passport assembly.
    #[serde(default)]
    pub visa_sources: Vec<VisaSourceConfig>,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind address host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Public base URL of this broker (no trailing slash).
    pub external_url: String,
    /// Deployment environment label (`prod`, `test`, `dev`, `staging`, `development`).
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "dev".to_string()
}

/// RS256 signing key configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SigningConfig {
    /// Path to a PEM-encoded RS256 private key.
    pub private_key_pem: String,
    /// Lifetime of minted Passport JWTs in seconds.
    pub passport_lifetime_seconds: u64,
    /// Lifetime of broker access tokens in seconds.
    pub token_lifetime_seconds: u64,
}

/// RP login session cookie configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionConfig {
    /// Environment variable holding the cookie signing secret.
    pub cookie_secret_env: String,
    /// RP session cookie lifetime in seconds.
    pub session_lifetime_seconds: u64,
}

/// Upstream OIDC provider the broker authenticates against as a Relying Party.
#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamIdpConfig {
    /// Short name used in `/login/{name}`.
    pub name: String,
    /// Upstream issuer URL for OIDC discovery.
    pub issuer: String,
    /// OAuth client identifier registered at the upstream IdP.
    pub client_id: String,
    /// Environment variable holding the upstream client secret.
    pub client_secret_env: String,
    /// OAuth scopes to request from the upstream IdP.
    pub scopes: Vec<String>,
    /// Maps broker identity fields to upstream JWT / userinfo claim names.
    #[serde(default)]
    pub claim_mapping: HashMap<String, String>,
}

/// Visa registry or other visa source queried during passport assembly.
#[derive(Debug, Clone, Deserialize)]
pub struct VisaSourceConfig {
    /// Human-readable source name.
    pub name: String,
    /// Base URL of the visa source service.
    pub url: String,
}

impl BrokerConfig {
    /// Load configuration from a TOML file, resolving `*_env` fields from the environment.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("BROKER").separator("__"))
            .build()?;
        settings.try_deserialize()
    }

    /// Resolve the RP session cookie secret from the configured environment variable.
    pub fn cookie_secret(&self) -> Result<String, std::env::VarError> {
        std::env::var(&self.session.cookie_secret_env)
    }

    /// Resolve an upstream client secret from its configured environment variable.
    pub fn upstream_client_secret(idp: &UpstreamIdpConfig) -> Result<String, std::env::VarError> {
        std::env::var(&idp.client_secret_env)
    }

    /// Public issuer URL for downstream OIDC metadata (same as `server.external_url`).
    pub fn issuer_url(&self) -> &str {
        self.server.external_url.trim_end_matches('/')
    }

    /// Callback URL registered with upstream IdPs for the authorization code flow.
    pub fn callback_url(&self) -> String {
        format!("{}/callback", self.issuer_url())
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
            port = 8080
            external_url = "https://aai.example.org"
            environment = "test"

            [signing]
            private_key_pem = "/secrets/broker_rs256.pem"
            passport_lifetime_seconds = 3600
            token_lifetime_seconds = 3600

            [session]
            cookie_secret_env = "BROKER_COOKIE_SECRET"
            session_lifetime_seconds = 600

            [[upstream_idps]]
            name = "my-institute"
            issuer = "https://idp.example.org/realms/main"
            client_id = "ga4gh-broker"
            client_secret_env = "MY_INSTITUTE_CLIENT_SECRET"
            scopes = ["openid", "profile", "email"]

            [upstream_idps.claim_mapping]
            sub = "sub"
            email = "email"
            affiliation = "eduperson_scoped_affiliation"

            [[visa_sources]]
            name = "local-registry"
            url = "http://visa-registry:8081"
        "#;

        let config: BrokerConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");
        assert_eq!(config.upstream_idps.len(), 1);
        assert_eq!(config.upstream_idps[0].name, "my-institute");
        assert_eq!(config.callback_url(), "https://aai.example.org/callback");
    }
}
