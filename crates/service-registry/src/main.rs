// SPDX-License-Identifier: Apache-2.0

//! Binary entrypoint for the GA4GH Service Registry.

use anyhow::Context;
use service_registry::{startup, RegistryConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/service-registry.toml".to_string());

    let config = RegistryConfig::load_from_file(&config_path)
        .with_context(|| format!("loading service registry config from {config_path}"))?;

    startup::validate_log_level(&config)?;
    startup::run(config).await
}
