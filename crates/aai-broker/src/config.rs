// SPDX-License-Identifier: Apache-2.0

//! Broker configuration loaded from TOML and environment variables.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Top-level broker configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BrokerConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
    /// Passport and access-token signing settings.
    pub signing: SigningConfig,
    /// Short-lived RP session cookie settings.
    pub session: SessionConfig,
    /// Upstream OIDC identity providers.
    pub upstream_idps: Vec<UpstreamIdpConfig>,
    /// Visa assertion sources queried during passport assembly.
    #[serde(default)]
    pub visa_sources: Vec<VisaSourceConfig>,
    /// Optional Access Decision Service integration.
    #[serde(default)]
    pub ads: Option<AdsIntegrationConfig>,
    /// Upstream TLS trust. Absent means bundled webpki roots only.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Optional flat claim and audience. Absent leaves passport bytes unchanged.
    #[serde(default)]
    pub token_claims: TokenClaimsConfig,
}

/// Extra trust anchors for upstream OIDC HTTP.
///
/// The broker client uses rustls with the bundled webpki roots. It does not
/// read the operating-system store or `SSL_CERT_FILE`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TlsConfig {
    /// PEM file of CA certificates added on top of the bundled webpki roots.
    /// Unset keeps those roots only.
    #[serde(default)]
    pub extra_ca_bundle: Option<String>,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind address host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Public base URL of this broker (no trailing slash).
    pub external_url: String,
    /// Deployment environment label (`prod`, `test`, `dev`, `staging`, `development`).
    #[serde(default = "default_environment")]
    pub environment: String,
    /// Exact origins (scheme + host + port) allowed as OAuth `return_url`.
    /// Empty in non-development rejects all `return_url` values.
    #[serde(default)]
    pub allowed_return_url_origins: Vec<String>,
    /// Max `/login` and `/callback` attempts per client IP per minute. `0` disables.
    #[serde(default = "default_login_rate_limit")]
    pub login_rate_limit_per_minute: u32,
    /// Optional JSON file for Passport issue/revoke persistence (shared volume in multi-replica).
    #[serde(default)]
    pub passport_ledger_path: Option<String>,
    /// Environment variable for `POST /revoke-passports` (`X-API-Key`).
    #[serde(default = "default_admin_api_key_env")]
    pub admin_api_key_env: String,
}

fn default_login_rate_limit() -> u32 {
    20
}

fn default_admin_api_key_env() -> String {
    "BROKER_ADMIN_API_KEY".to_string()
}

fn default_environment() -> String {
    "dev".to_string()
}

/// RS256 signing key configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SigningConfig {
    /// Path to a PEM-encoded RS256 private key.
    pub private_key_pem: String,
    /// Additional PEMs (public or private) published in JWKS during rotation overlap.
    #[serde(default)]
    pub previous_key_pems: Vec<String>,
    /// Lifetime of minted Passport JWTs in seconds.
    pub passport_lifetime_seconds: u64,
    /// Lifetime of broker access tokens in seconds.
    pub token_lifetime_seconds: u64,
}

/// RP login session cookie configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionConfig {
    /// Environment variable holding the cookie signing secret.
    pub cookie_secret_env: String,
    /// RP session cookie lifetime in seconds.
    pub session_lifetime_seconds: u64,
    /// When unset, derived from `server.external_url` (`https` → true, else false).
    #[serde(default)]
    pub secure_cookies: Option<bool>,
}

/// Upstream OIDC provider the broker authenticates against as a Relying Party.
#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamIdpConfig {
    /// Short name used in `/login/{name}`.
    pub name: String,
    /// Upstream issuer URL for OIDC discovery.
    pub issuer: String,
    /// OAuth client identifier registered at the upstream IdP.
    pub client_id: String,
    /// Environment variable holding the upstream client secret.
    pub client_secret_env: String,
    /// OAuth scopes to request from the upstream IdP.
    pub scopes: Vec<String>,
    /// Maps broker identity fields to upstream JWT / userinfo claim names.
    #[serde(default)]
    pub claim_mapping: HashMap<String, String>,
}

/// Visa registry or other visa source queried during passport assembly.
#[derive(Debug, Clone, Deserialize)]
pub struct VisaSourceConfig {
    /// Human-readable source name.
    pub name: String,
    /// Base URL of the visa source service.
    pub url: String,
    /// Environment variable holding the API key for `GET /visas` (default: `REGISTRY_BOOTSTRAP_API_KEY`).
    #[serde(default = "default_visa_source_api_key_env")]
    pub api_key_env: String,
    /// When true, a failed fetch aborts Passport issuance.
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_visa_source_api_key_env() -> String {
    "REGISTRY_BOOTSTRAP_API_KEY".to_string()
}

fn default_true() -> bool {
    true
}

/// ADS integration for researcher sync and signed visa export.
#[derive(Debug, Clone, Deserialize)]
pub struct AdsIntegrationConfig {
    /// Base URL of the Access Decision Service (no trailing slash).
    pub url: String,
    /// Environment variable holding the DAC API key for sync and signed-visas.
    #[serde(default = "default_ads_api_key_env")]
    pub sync_api_key_env: String,
}

fn default_ads_api_key_env() -> String {
    "ADS_DAC_API_KEY".to_string()
}

fn default_claim_name() -> String {
    "groups".to_string()
}

fn default_visa_type() -> String {
    "AffiliationAndRole".to_string()
}

/// Flat claim taken from signature-checked visas, plus the shared audience.
///
/// Both switches default off. Off keeps today's passport: upstream `groups`,
/// no `aud`, and visa strings embedded without a signature check.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenClaimsConfig {
    /// When true, fill `claim_name` only from checked visas of `visa_type`.
    #[serde(default)]
    pub enabled: bool,
    /// Passport claim that receives the visa values. Default `groups`.
    #[serde(default = "default_claim_name")]
    pub claim_name: String,
    /// `ga4gh_visa_v1.type` whose `value` is copied into the flat claim.
    #[serde(default = "default_visa_type")]
    pub visa_type: String,
    /// At most one audience. Empty omits `aud`. Written only when `enabled`.
    #[serde(default)]
    pub audiences: Vec<String>,
    /// When true, drop visa JWTs that fail the signature check before embedding.
    /// Default off. Example configs leave it off.
    #[serde(default)]
    pub verify_embedded_visas: bool,
    /// Visa-issuer JWKS file. Mutually exclusive with `jwks_url`.
    #[serde(default)]
    pub jwks_file: Option<String>,
    /// Visa-issuer JWKS URL, fetched at startup. Mutually exclusive with `jwks_file`.
    #[serde(default)]
    pub jwks_url: Option<String>,
}

impl Default for TokenClaimsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            claim_name: default_claim_name(),
            visa_type: default_visa_type(),
            audiences: Vec::new(),
            verify_embedded_visas: false,
            jwks_file: None,
            jwks_url: None,
        }
    }
}

impl TokenClaimsConfig {
    /// True when startup must load the visa-issuer JWKS.
    pub fn needs_verifier(&self) -> bool {
        self.enabled || self.verify_embedded_visas
    }

    /// Reject a config that would mint an ambiguous or unchecked token.
    pub fn validate(&self) -> Result<(), String> {
        if self.audiences.len() > 1 {
            return Err(
                "token_claims.audiences accepts at most one audience; that audience is shared by every resource server that accepts it, so a token valid for one is valid for the others"
                    .to_string(),
            );
        }
        if self.audiences.iter().any(|aud| aud.trim().is_empty()) {
            return Err("token_claims.audiences contains an empty audience".to_string());
        }
        if self.claim_name.trim().is_empty() {
            return Err("token_claims.claim_name is empty".to_string());
        }
        const RESERVED: &[&str] = &[
            "sub",
            "iss",
            "aud",
            "exp",
            "iat",
            "jti",
            "nbf",
            "scope",
            "email",
            "name",
            "ga4gh_passport_v1",
        ];
        if RESERVED.contains(&self.claim_name.as_str()) {
            return Err(format!(
                "token_claims.claim_name {} collides with a passport claim",
                self.claim_name
            ));
        }
        if self.visa_type.trim().is_empty() {
            return Err("token_claims.visa_type is empty".to_string());
        }
        if self.needs_verifier() {
            match (self.jwks_file.as_deref(), self.jwks_url.as_deref()) {
                (Some(file), None) if !file.trim().is_empty() => {}
                (None, Some(url)) if !url.trim().is_empty() => {}
                (Some(_), Some(_)) => {
                    return Err("token_claims accepts jwks_file or jwks_url, not both".to_string());
                }
                _ => {
                    return Err(
                        "token_claims.enabled or verify_embedded_visas requires jwks_file or jwks_url"
                            .to_string(),
                    );
                }
            }
        }
        Ok(())
    }
}

impl BrokerConfig {
    /// Load configuration from a TOML file, resolving `*_env` fields from the environment.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref();
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("BROKER").separator("__"))
            .build()?;
        settings.try_deserialize()
    }

    /// Resolve the RP session cookie secret from the configured environment variable.
    pub fn cookie_secret(&self) -> Result<String, std::env::VarError> {
        std::env::var(&self.session.cookie_secret_env)
    }

    /// Whether RP session cookies include the `Secure` attribute.
    pub fn secure_cookies(&self) -> bool {
        self.session
            .secure_cookies
            .unwrap_or_else(|| self.server.external_url.starts_with("https://"))
    }

    /// Resolve an upstream client secret from its configured environment variable.
    pub fn upstream_client_secret(idp: &UpstreamIdpConfig) -> Result<String, std::env::VarError> {
        std::env::var(&idp.client_secret_env)
    }

    /// Public issuer URL for downstream OIDC metadata (same as `server.external_url`).
    pub fn issuer_url(&self) -> &str {
        self.server.external_url.trim_end_matches('/')
    }

    /// Admin API key used for `POST /revoke-passports`.
    pub fn admin_api_key(&self) -> Result<String, std::env::VarError> {
        std::env::var(&self.server.admin_api_key_env)
    }

    /// Callback URL registered with upstream IdPs for the authorization code flow.
    pub fn callback_url(&self) -> String {
        format!("{}/callback", self.issuer_url())
    }

    /// Returns `true` when the deployment is explicitly marked as development.
    pub fn is_development(&self) -> bool {
        matches!(
            self.server.environment.as_str(),
            "development" | "dev" | "local"
        )
    }

    /// Known-insecure bootstrap secrets that must never run outside development.
    pub fn reject_insecure_bootstrap_secrets(&self) -> Result<(), String> {
        if self.is_development() || std::env::var("GA4GH_ALLOW_DEV_SECRETS").is_ok() {
            return Ok(());
        }
        let Ok(secret) = self.cookie_secret() else {
            return Ok(());
        };
        const BLOCKED: &[&str] = &[
            "dev-broker-cookie-secret",
            "change-me",
            "secret",
            "password",
        ];
        if BLOCKED.iter().any(|blocked| secret == *blocked) {
            return Err(
                "BROKER_COOKIE_SECRET is a documented development value; set a unique secret before production use"
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Audience and visa-verifier rules from ADR-004. Runs even when the feature is off.
    pub fn validate_token_claims(&self) -> Result<(), String> {
        self.token_claims.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example_config_shape() {
        let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 8080
            external_url = "https://aai.example.org"
            environment = "test"

            [signing]
            private_key_pem = "/secrets/broker_rs256.pem"
            passport_lifetime_seconds = 3600
            token_lifetime_seconds = 3600

            [session]
            cookie_secret_env = "BROKER_COOKIE_SECRET"
            session_lifetime_seconds = 600

            [[upstream_idps]]
            name = "my-institute"
            issuer = "https://idp.example.org/realms/main"
            client_id = "ga4gh-broker"
            client_secret_env = "MY_INSTITUTE_CLIENT_SECRET"
            scopes = ["openid", "profile", "email"]

            [upstream_idps.claim_mapping]
            sub = "sub"
            email = "email"
            affiliation = "eduperson_scoped_affiliation"

            [[visa_sources]]
            name = "local-registry"
            url = "http://visa-registry:8081"
        "#;

        let config: BrokerConfig = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .expect("build config")
            .try_deserialize()
            .expect("parse config");
        assert_eq!(config.upstream_idps.len(), 1);
        assert_eq!(config.upstream_idps[0].name, "my-institute");
        assert_eq!(config.callback_url(), "https://aai.example.org/callback");
        assert_eq!(
            config.visa_sources[0].api_key_env,
            "REGISTRY_BOOTSTRAP_API_KEY"
        );
        assert!(config.visa_sources[0].required);
        assert!(!config.token_claims.enabled);
        assert!(!config.token_claims.verify_embedded_visas);
        assert!(config.token_claims.audiences.is_empty());
        assert!(config.validate_token_claims().is_ok());
    }

    #[test]
    fn two_audiences_fail_even_when_the_feature_is_off() {
        let claims = TokenClaimsConfig {
            audiences: vec![
                "https://a.example".to_string(),
                "https://b.example".to_string(),
            ],
            ..TokenClaimsConfig::default()
        };
        assert!(claims.validate().is_err());
    }

    #[test]
    fn one_audience_is_accepted_and_a_missing_jwks_is_rejected_only_when_needed() {
        let claims = TokenClaimsConfig {
            audiences: vec!["https://resources.example".to_string()],
            ..TokenClaimsConfig::default()
        };
        assert!(claims.validate().is_ok());
        let claims = TokenClaimsConfig {
            enabled: true,
            audiences: vec!["https://resources.example".to_string()],
            ..TokenClaimsConfig::default()
        };
        assert!(claims.validate().is_err());
        let claims = TokenClaimsConfig {
            enabled: true,
            audiences: vec!["https://resources.example".to_string()],
            jwks_file: Some("/secrets/visa.jwks.json".to_string()),
            ..TokenClaimsConfig::default()
        };
        assert!(claims.validate().is_ok());
        let claims = TokenClaimsConfig {
            enabled: true,
            audiences: vec!["https://resources.example".to_string()],
            jwks_file: Some("/secrets/visa.jwks.json".to_string()),
            jwks_url: Some("https://visa.example/jwks.json".to_string()),
            ..TokenClaimsConfig::default()
        };
        assert!(claims.validate().is_err());
    }
}
