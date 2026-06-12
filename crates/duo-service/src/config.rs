// SPDX-License-Identifier: Apache-2.0

//! DUO service configuration loaded from TOML and environment variables.

use std::path::Path;

use serde::Deserialize;

/// Top-level DUO service configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DuoServiceConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
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

impl DuoServiceConfig {
    /// Load configuration from a TOML file, with optional `DUO__` environment overrides.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("DUO").separator("__"))
            .build()?;
        settings.try_deserialize()
    }

    /// Public base URL for service metadata.
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
            port = 8082
            external_url = "https://duo.example.org"
            environment = "test"
        "#;

        let config: DuoServiceConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");

        assert_eq!(config.server.port, 8082);
        assert_eq!(config.external_url(), "https://duo.example.org");
    }
}
