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
    /// IdP groups that may operate the DAC queue (in addition to admins).
    /// Empty means only the admin claim may approve or reject requests.
    #[serde(default)]
    pub dac_operator_groups: Vec<String>,
    /// Deployment environment (`development`, `dev`, `prod`, …).
    #[serde(default = "default_environment")]
    pub environment: String,
    /// When unset, derived from `public_base_url` (`https` → true).
    #[serde(default)]
    pub secure_cookies: Option<bool>,
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

fn default_environment() -> String {
    "development".to_string()
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
        if !self.is_development() && std::env::var("GA4GH_ALLOW_DEV_SECRETS").is_err() {
            const BLOCKED: &[&str] = &[
                "dev-admin-ui-session-secret-min-32-chars",
                "dev-ads-api-key",
                "dev-service-registry-key",
            ];
            if BLOCKED
                .iter()
                .any(|blocked| self.session_secret == *blocked)
                || BLOCKED
                    .iter()
                    .any(|blocked| self.ads_dac_api_key == *blocked)
            {
                anyhow::bail!(
                    "admin-ui is using a documented development secret; set unique session_secret and ads_dac_api_key"
                );
            }
        }
        Ok(())
    }

    pub fn is_development(&self) -> bool {
        matches!(self.environment.as_str(), "development" | "dev" | "local")
    }

    pub fn secure_cookies(&self) -> bool {
        self.secure_cookies
            .unwrap_or_else(|| self.public_base_url.starts_with("https://"))
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
