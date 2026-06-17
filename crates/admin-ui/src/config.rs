use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AdminUiConfig {
    pub listen_addr: String,
    pub public_base_url: String,
    /// Base URL for server-side broker API calls (JWKS, health).
    pub broker_base_url: String,
    /// Browser-facing broker URL for OIDC login redirects. Defaults to `broker_base_url`.
    #[serde(default)]
    pub broker_public_url: Option<String>,
    pub ads_base_url: String,
    pub ads_dac_api_key: String,
    pub duo_base_url: String,
    pub visa_registry_base_url: String,
    pub service_registry_base_url: String,
    #[serde(default = "default_agreement_registry_base_url")]
    pub agreement_registry_base_url: String,
    pub session_secret: String,
    #[serde(default = "default_session_ttl_hours")]
    pub session_ttl_hours: u64,
    #[serde(default = "default_admin_claim")]
    pub admin_claim: String,
    #[serde(default = "default_admin_claim_value")]
    pub admin_claim_value: String,
    #[serde(default)]
    pub static_dir: Option<PathBuf>,
    /// Service registry registration key (Admin service management).
    #[serde(default)]
    pub service_registry_registration_key: Option<String>,
    /// Hint shown on System page for broker config file location.
    #[serde(default = "default_broker_config_path")]
    pub broker_config_path: String,
    /// Optional ISO-8601 date when broker signing keys should be rotated (dashboard warning).
    #[serde(default)]
    pub signing_key_rotation_due: Option<String>,
}

fn default_broker_config_path() -> String {
    "docker/config/broker.toml".to_string()
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

fn default_agreement_registry_base_url() -> String {
    "http://localhost:8086".to_string()
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

    /// Broker URL shown in the browser (login redirect). In Docker, set to `http://localhost:8080`
    /// while `broker_base_url` stays on the internal service name.
    pub fn broker_public_url(&self) -> &str {
        self.broker_public_url
            .as_deref()
            .unwrap_or(self.broker_base_url.as_str())
    }
}
