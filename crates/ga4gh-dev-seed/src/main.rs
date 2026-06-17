// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for seeding local / test stacks.

use anyhow::Context;
use clap::Parser;
use ga4gh_dev_seed::{seed_dev_stack, SeedConfig, SeedProfile};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "seed-dev-stack",
    about = "Load idempotent demo data into a running GA4GH dev/test stack"
)]
struct Args {
    /// Compose profile: postgres (default stack) or sqlite (818x/819x ports).
    #[arg(long, env = "GA4GH_SEED_PROFILE", default_value = "postgres")]
    profile: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("ga4gh_dev_seed=info".parse()?),
        )
        .init();

    let args = Args::parse();
    let profile = SeedProfile::parse(&args.profile)?;
    let config = SeedConfig::from_profile(profile);
    let summary = seed_dev_stack(&config).await.context("seed dev stack")?;

    println!("Dev stack seed complete ({profile:?} profile):");
    println!("  services registered: {}", summary.services_registered);
    println!(
        "  datasets: {} created, {} skipped",
        summary.datasets_created, summary.datasets_skipped
    );
    println!(
        "  projects: {} created, {} skipped",
        summary.projects_created, summary.projects_skipped
    );
    println!(
        "  pending DAC requests: {} created, {} skipped",
        summary.pending_requests_created, summary.pending_requests_skipped
    );
    println!(
        "  grants: {} created, {} skipped",
        summary.grants_created, summary.grants_skipped
    );
    println!(
        "  visas: {} created, {} skipped",
        summary.visas_created, summary.visas_skipped
    );
    println!();
    println!("Admin UI: {}", config.admin_ui_url);
    println!("Researcher login subject: {}", config.researcher_sub);

    Ok(())
}
