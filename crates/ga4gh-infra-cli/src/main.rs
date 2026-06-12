// SPDX-License-Identifier: Apache-2.0

//! Combined `ga4gh-infra` CLI entrypoint.

use std::path::PathBuf;

use access_decision_service::AdsConfig;
use aai_broker::BrokerConfig;
use anyhow::Context;
use clap::{Parser, Subcommand};
use duo_service::DuoServiceConfig;
use ga4gh_infra_cli::{generate_default_keys, generate_pem, run_all_in_one, AllInOneConfig};
use service_registry::RegistryConfig as ServiceRegistryConfig;
use tracing_subscriber::EnvFilter;
use visa_registry::RegistryConfig as VisaRegistryConfig;

/// GA4GH infrastructure services (broker, visa registry, DUO, service registry, ADS).
#[derive(Parser)]
#[command(name = "ga4gh-infra", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the GA4GH AAI broker (OIDC Relying Party).
    Broker {
        /// Path to broker TOML configuration.
        #[arg(long, value_name = "FILE")]
        config: PathBuf,
    },
    /// Run the GA4GH Visa Registry.
    #[command(name = "visa-registry")]
    VisaRegistry {
        /// Path to visa registry TOML configuration.
        #[arg(long, value_name = "FILE")]
        config: PathBuf,
    },
    /// Run the GA4GH DUO matching service.
    #[command(name = "duo-service")]
    DuoService {
        /// Path to DUO service TOML configuration.
        #[arg(long, value_name = "FILE")]
        config: PathBuf,
    },
    /// Run the GA4GH Service Registry.
    #[command(name = "service-registry")]
    ServiceRegistry {
        /// Path to service registry TOML configuration.
        #[arg(long, value_name = "FILE")]
        config: PathBuf,
    },
    /// Run the GA4GH Access Decision Service (ADS).
    #[command(name = "access-decision-service")]
    AccessDecisionService {
        /// Path to ADS TOML configuration.
        #[arg(long, value_name = "FILE")]
        config: PathBuf,
    },
    /// Run broker, visa-registry, duo-service, service-registry, and access-decision-service in one process.
    #[command(name = "all-in-one")]
    AllInOne {
        /// Path to combined all-in-one TOML configuration.
        #[arg(long, value_name = "FILE")]
        config: PathBuf,
    },
    /// Generate RS256 PKCS#8 PEM signing keys (broker / visa-registry).
    Keygen {
        /// Write a single key to this path (fails if the file already exists).
        #[arg(long, value_name = "FILE", conflicts_with = "output_dir")]
        output: Option<PathBuf>,
        /// Write default broker and registry keys when missing.
        #[arg(long, value_name = "DIR", conflicts_with = "output")]
        output_dir: Option<PathBuf>,
        /// RSA modulus size in bits (minimum 2048).
        #[arg(long, default_value_t = 2048)]
        bits: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    match Cli::parse().command {
        Commands::Broker { config } => {
            let cfg = BrokerConfig::load_from_file(&config)
                .with_context(|| format!("loading broker config from {}", config.display()))?;
            aai_broker::validate_log_level(&cfg).map_err(anyhow::Error::msg)?;
            aai_broker::run(cfg).await
        }
        Commands::VisaRegistry { config } => {
            let cfg = VisaRegistryConfig::load_from_file(&config).with_context(|| {
                format!("loading visa registry config from {}", config.display())
            })?;
            visa_registry::validate_log_level(&cfg).map_err(anyhow::Error::msg)?;
            visa_registry::run(cfg).await
        }
        Commands::DuoService { config } => {
            let cfg = DuoServiceConfig::load_from_file(&config)
                .with_context(|| format!("loading duo service config from {}", config.display()))?;
            duo_service::validate_log_level(&cfg).map_err(anyhow::Error::msg)?;
            duo_service::run(cfg).await
        }
        Commands::ServiceRegistry { config } => {
            let cfg = ServiceRegistryConfig::load_from_file(&config).with_context(|| {
                format!("loading service registry config from {}", config.display())
            })?;
            service_registry::validate_log_level(&cfg).map_err(anyhow::Error::msg)?;
            service_registry::run(cfg).await
        }
        Commands::AccessDecisionService { config } => {
            let cfg = AdsConfig::load_from_file(&config)
                .with_context(|| format!("loading ADS config from {}", config.display()))?;
            access_decision_service::validate_log_level(&cfg).map_err(anyhow::Error::msg)?;
            access_decision_service::run(cfg).await
        }
        Commands::AllInOne { config } => {
            let cfg = AllInOneConfig::load_from_file(&config)
                .with_context(|| format!("loading all-in-one config from {}", config.display()))?;
            run_all_in_one(cfg).await
        }
        Commands::Keygen {
            output,
            output_dir,
            bits,
        } => run_keygen(output, output_dir, bits),
    }
}

fn run_keygen(
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    bits: usize,
) -> anyhow::Result<()> {
    match (output, output_dir) {
        (Some(path), None) => {
            generate_pem(&path, bits)?;
            println!("Wrote {}", path.display());
        }
        (None, Some(dir)) => {
            let written = generate_default_keys(&dir, bits)?;
            if written.is_empty() {
                println!("All default keys already exist in {}", dir.display());
            } else {
                for path in written {
                    println!("Wrote {}", path.display());
                }
            }
        }
        (None, None) => {
            anyhow::bail!("specify --output FILE or --output-dir DIR");
        }
        (Some(_), Some(_)) => unreachable!("clap conflicts_with prevents both"),
    }
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();
}
