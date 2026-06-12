// SPDX-License-Identifier: Apache-2.0

//! Visa registry configuration loaded from TOML and environment variables.

use std::path::Path;

use serde::Deserialize;

/// Top-level visa registry configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
    /// Visa JWT signing settings.
    pub signing: SigningConfig,
    /// Database connection settings.
    pub database: DatabaseConfig,
    /// DAC API key authentication settings.
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
    /// Deployment environment label (`prod`, `test`, `dev`, `staging`, `development`).
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "dev".to_string()
}

/// RS256 signing key configuration for visa JWT issuance.
#[derive(Debug, Clone, Deserialize)]
pub struct SigningConfig {
    /// Path to a PEM-encoded RS256 private key.
    pub private_key_pem: String,
    /// Lifetime of minted visa JWTs in seconds.
    pub visa_lifetime_seconds: u64,
}

/// Supported database backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    /// PostgreSQL (production default).
    #[default]
    Postgres,
    /// SQLite file database (desktop/demo).
    Sqlite,
}

/// Database persistence configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Database driver (`postgres` or `sqlite`).
    #[serde(default)]
    pub driver: DatabaseDriver,
    /// Inline connection URL (`postgres://...` or `sqlite:///path/to/db.sqlite`).
    #[serde(default)]
    pub url: Option<String>,
    /// Environment variable holding the connection URL when `url` is not set inline.
    #[serde(default = "default_url_env")]
    pub url_env: String,
    /// Run embedded migrations on startup (PostgreSQL only; SQLite always migrates).
    #[serde(default)]
    pub auto_migrate: bool,
}

fn default_url_env() -> String {
    "REGISTRY_DATABASE_URL".to_string()
}

/// DAC API key authentication configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Environment variable for a bootstrap API key registered on first startup.
    pub bootstrap_api_key_env: String,
}

impl RegistryConfig {
    /// Load configuration from a TOML file, resolving `*_env` fields from the environment.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("REGISTRY").separator("__"))
            .build()?;
        settings.try_deserialize()
    }

    /// Resolve the database connection URL from inline config or environment.
    pub fn database_url(&self) -> Result<String, crate::error::RegistryError> {
        if let Some(url) = &self.database.url {
            if url.trim().is_empty() {
                return Err(crate::error::RegistryError::Config(
                    "database.url must not be empty".to_string(),
                ));
            }
            return Ok(url.clone());
        }

        std::env::var(&self.database.url_env).map_err(|err| {
            crate::error::RegistryError::Config(format!(
                "missing database URL env `{}`: {err}",
                self.database.url_env
            ))
        })
    }

    /// Resolve the optional bootstrap API key from the configured environment variable.
    pub fn bootstrap_api_key(&self) -> Result<String, std::env::VarError> {
        std::env::var(&self.auth.bootstrap_api_key_env)
    }

    /// Public issuer URL for visa JWTs (same as `server.external_url`).
    pub fn issuer_url(&self) -> &str {
        self.server.external_url.trim_end_matches('/')
    }

    /// JWKS URL served by this registry.
    pub fn jwks_url(&self) -> String {
        format!("{}/jwks.json", self.issuer_url())
    }

    /// Returns `true` when the deployment is explicitly marked as development.
    pub fn is_development(&self) -> bool {
        matches!(
            self.server.environment.as_str(),
            "development" | "dev" | "local"
        )
    }

    /// Whether migrations should run automatically on startup.
    pub fn should_auto_migrate(&self) -> bool {
        match self.database.driver {
            DatabaseDriver::Sqlite => true,
            DatabaseDriver::Postgres => self.database.auto_migrate,
        }
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
            port = 8081
            external_url = "https://visas.example.org"
            environment = "test"

            [signing]
            private_key_pem = "/secrets/registry_rs256.pem"
            visa_lifetime_seconds = 86400

            [database]
            driver = "postgres"
            url_env = "REGISTRY_DATABASE_URL"

            [auth]
            bootstrap_api_key_env = "REGISTRY_BOOTSTRAP_API_KEY"
        "#;

        let config: RegistryConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");

        assert_eq!(config.server.port, 8081);
        assert_eq!(config.database.driver, DatabaseDriver::Postgres);
        assert!(!config.should_auto_migrate());
        assert_eq!(config.issuer_url(), "https://visas.example.org");
    }

    #[test]
    fn sqlite_defaults_to_auto_migrate() {
        let toml = r#"
            [server]
            host = "127.0.0.1"
            port = 8081
            external_url = "http://localhost:8081"
            environment = "development"

            [signing]
            private_key_pem = "/secrets/registry.pem"
            visa_lifetime_seconds = 86400

            [database]
            driver = "sqlite"
            url = "sqlite:///tmp/visa_registry.sqlite"

            [auth]
            bootstrap_api_key_env = "REGISTRY_BOOTSTRAP_API_KEY"
        "#;

        let config: RegistryConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");

        assert_eq!(config.database.driver, DatabaseDriver::Sqlite);
        assert!(config.should_auto_migrate());
    }
}
