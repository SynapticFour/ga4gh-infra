// SPDX-License-Identifier: Apache-2.0

//! Environment-driven configuration for stack seeding.

use std::env;

/// Known compose port layouts exposed on the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedProfile {
    /// `docker/docker-compose.yml` (default local stack).
    Postgres,
    /// `docker/docker-compose.sqlite.yml`.
    Sqlite,
}

impl SeedProfile {
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "postgres" | "default" | "local" => Ok(Self::Postgres),
            "sqlite" => Ok(Self::Sqlite),
            other => anyhow::bail!("unknown seed profile `{other}` (use postgres or sqlite)"),
        }
    }
}

/// Host-facing service URLs and dev API keys for seeding.
#[derive(Debug, Clone)]
pub struct SeedConfig {
    pub profile: SeedProfile,
    pub broker_url: String,
    pub ads_url: String,
    pub service_registry_url: String,
    pub visa_registry_url: String,
    pub sample_resource_url: String,
    pub agreement_registry_url: String,
    pub admin_ui_url: String,
    pub ads_api_key: String,
    pub service_registry_key: String,
    pub visa_api_key: String,
    pub researcher_sub: String,
}

impl SeedConfig {
    pub fn from_profile(profile: SeedProfile) -> Self {
        let (broker, visa, duo, registry, ads, sample, agreement, admin) = match profile {
            SeedProfile::Postgres => (
                "http://localhost:8080",
                "http://localhost:8081",
                "http://localhost:8082",
                "http://localhost:8083",
                "http://localhost:8090",
                "http://localhost:8084",
                "http://localhost:8086",
                "http://localhost:8095",
            ),
            SeedProfile::Sqlite => (
                "http://localhost:8180",
                "http://localhost:8181",
                "http://localhost:8182",
                "http://localhost:8183",
                "http://localhost:8190",
                "http://localhost:8184",
                "http://localhost:8186",
                "http://localhost:8195",
            ),
        };

        let _ = duo; // registered in service list; no direct seed calls today.

        Self {
            profile,
            broker_url: env_or("GA4GH_BROKER_URL", broker),
            ads_url: env_or("GA4GH_ADS_URL", ads),
            service_registry_url: env_or("GA4GH_SERVICE_REGISTRY_URL", registry),
            visa_registry_url: env_or("GA4GH_VISA_REGISTRY_URL", visa),
            sample_resource_url: env_or("GA4GH_SAMPLE_RESOURCE_URL", sample),
            agreement_registry_url: env_or("GA4GH_AGREEMENT_REGISTRY_URL", agreement),
            admin_ui_url: env_or("GA4GH_ADMIN_UI_URL", admin),
            ads_api_key: env_or("GA4GH_ADS_API_KEY", "dev-ads-api-key"),
            service_registry_key: env_or(
                "GA4GH_SERVICE_REGISTRY_REGISTRATION_KEY",
                "dev-service-registry-key",
            ),
            visa_api_key: env_or("GA4GH_VISA_API_KEY", "dev-visa-api-key"),
            researcher_sub: env_or("MOCK_IDP_SUBJECT", "researcher@uni-heidelberg.de"),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let profile = SeedProfile::parse(&env_or("GA4GH_SEED_PROFILE", "postgres"))?;
        Ok(Self::from_profile(profile))
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_profile_uses_co_deploy_ports() {
        let cfg = SeedConfig::from_profile(SeedProfile::Sqlite);
        assert_eq!(cfg.broker_url, "http://localhost:8180");
        assert_eq!(cfg.ads_url, "http://localhost:8190");
        assert_eq!(cfg.admin_ui_url, "http://localhost:8195");
    }
}
