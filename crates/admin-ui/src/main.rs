use std::path::PathBuf;

use admin_ui::AdminUiConfig;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "admin-ui", about = "GA4GH infrastructure admin dashboard")]
struct Args {
    /// Path to TOML configuration file.
    #[arg(long, env = "ADMIN_UI_CONFIG")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("admin_ui=info".parse()?))
        .init();

    let args = Args::parse();
    let config = AdminUiConfig::from_file(args.config.to_str().expect("utf-8 config path"))?;
    admin_ui::run(config).await
}
