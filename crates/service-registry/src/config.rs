// SPDX-License-Identifier: Apache-2.0

//! Service registry configuration loaded from TOML and environment variables.

use std::path::Path;

use serde::Deserialize;

/// Top-level service registry configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
    /// PostgreSQL connection settings.
    pub database: DatabaseConfig,
    /// Internal registration authentication settings.
    pub auth: AuthConfig,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind address host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Public base URL of this registry (no trailing slash).
    pub external_url: String,
    /// Deployment environment label.
    #[serde(default = "default_environment")]
    pub environment: String,
    /// When true, registration write endpoints are disabled.
    #[serde(default)]
    pub read_only: bool,
}

fn default_environment() -> String {
    "dev".to_string()
}

/// PostgreSQL persistence configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Environment variable holding the PostgreSQL connection URL.
    pub url_env: String,
}

/// Internal service registration authentication configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Environment variable holding the shared registration API key.
    pub registration_api_key_env: String,
}

impl RegistryConfig {
    /// Load configuration from a TOML file, with optional `SERVICE_REGISTRY__` overrides.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("SERVICE_REGISTRY").separator("__"))
            .build()?;
        settings.try_deserialize()
    }

    /// Resolve the PostgreSQL connection URL from the configured environment variable.
    pub fn database_url(&self) -> Result<String, std::env::VarError> {
        std::env::var(&self.database.url_env)
    }

    /// Resolve the internal registration API key from the configured environment variable.
    pub fn registration_api_key(&self) -> Result<String, std::env::VarError> {
        std::env::var(&self.auth.registration_api_key_env)
    }

    /// Public base URL for registry metadata.
    pub fn external_url(&self) -> &str {
        self.server.external_url.trim_end_matches('/')
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
            port = 8083
            external_url = "https://registry.example.org"
            environment = "test"
            read_only = true

            [database]
            url_env = "SERVICE_REGISTRY_DATABASE_URL"

            [auth]
            registration_api_key_env = "SERVICE_REGISTRY_REGISTRATION_KEY"
        "#;

        let config: RegistryConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");

        assert!(config.server.read_only);
        assert_eq!(config.external_url(), "https://registry.example.org");
    }
}
