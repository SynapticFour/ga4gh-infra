// SPDX-License-Identifier: Apache-2.0

//! Binary entrypoint for the sample GA4GH resource service.

use anyhow::Context;
use sample_resource::{startup, SampleResourceConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_path = std::env::args()
        .nth(1)
        .context("usage: sample-resource <config.toml>")?;
    let config =
        SampleResourceConfig::load_from_file(&config_path).context("load configuration")?;
    startup::validate_log_level(&config).map_err(anyhow::Error::msg)?;
    startup::run(config).await
}
