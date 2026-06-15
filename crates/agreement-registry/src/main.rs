// SPDX-License-Identifier: Apache-2.0

//! Binary entrypoint for the agreement registry service.

use agreement_registry::{startup, AgreementRegistryConfig};
use anyhow::Context;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/agreement-registry.example.toml".to_string());

    let config = AgreementRegistryConfig::load_from_file(&config_path)
        .with_context(|| format!("loading agreement registry config from {config_path}"))?;

    startup::validate_log_level(&config).map_err(anyhow::Error::msg)?;
    startup::run(config).await
}
