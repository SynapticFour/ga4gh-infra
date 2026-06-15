// SPDX-License-Identifier: Apache-2.0

//! Agreement registry configuration.

use std::path::Path;

use serde::Deserialize;

/// Top-level agreement registry configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AgreementRegistryConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub external_url: String,
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "dev".to_string()
}

impl AgreementRegistryConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("AGREEMENT_REGISTRY").separator("__"))
            .build()?;
        settings.try_deserialize()
    }

    pub fn external_url(&self) -> &str {
        self.server.external_url.trim_end_matches('/')
    }

    pub fn is_development(&self) -> bool {
        matches!(
            self.server.environment.as_str(),
            "development" | "dev" | "local"
        )
    }
}
