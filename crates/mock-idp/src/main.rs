// SPDX-License-Identifier: Apache-2.0

//! Minimal OIDC provider for docker-compose and end-to-end tests.

use mock_idp::{run, MockIdpConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    run(MockIdpConfig::from_env()).await
}
