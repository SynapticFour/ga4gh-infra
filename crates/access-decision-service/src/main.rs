// SPDX-License-Identifier: Apache-2.0

//! Standalone ADS binary.

use std::env;
use std::path::PathBuf;

use access_decision_service::{run, validate_log_level, AdsConfig};
use anyhow::Context;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: access-decision-service <config.toml>")?;

    let cfg = AdsConfig::load_from_file(&config_path)
        .with_context(|| format!("loading ADS config from {}", config_path.display()))?;
    validate_log_level(&cfg).map_err(anyhow::Error::msg)?;
    run(cfg).await
}
