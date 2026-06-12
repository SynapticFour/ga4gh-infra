// SPDX-License-Identifier: Apache-2.0

//! Binary entrypoint for the DUO service.

use anyhow::Context;
use duo_service::{startup, DuoServiceConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/duo-service.toml".to_string());

    let config = DuoServiceConfig::load_from_file(&config_path)
        .with_context(|| format!("loading duo service config from {config_path}"))?;

    startup::validate_log_level(&config)?;
    startup::run(config).await
}
