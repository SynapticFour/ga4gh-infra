use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AdminUiConfig {
    pub listen_addr: String,
    pub public_base_url: String,
    pub broker_base_url: String,
    pub ads_base_url: String,
    pub ads_dac_api_key: String,
    pub duo_base_url: String,
    pub visa_registry_base_url: String,
    pub service_registry_base_url: String,
    pub session_secret: String,
    #[serde(default = "default_session_ttl_hours")]
    pub session_ttl_hours: u64,
    #[serde(default = "default_admin_claim")]
    pub admin_claim: String,
    #[serde(default = "default_admin_claim_value")]
    pub admin_claim_value: String,
    #[serde(default)]
    pub static_dir: Option<PathBuf>,
}

fn default_session_ttl_hours() -> u64 {
    24
}

fn default_admin_claim() -> String {
    "groups".to_string()
}

fn default_admin_claim_value() -> String {
    "ga4gh-infra-admins".to_string()
}

impl AdminUiConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.session_secret.len() < 32 {
            anyhow::bail!("session_secret must be at least 32 characters");
        }
        Ok(())
    }

    pub fn session_ttl(&self) -> Duration {
        Duration::from_secs(self.session_ttl_hours * 3600)
    }
}
