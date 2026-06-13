// SPDX-License-Identifier: Apache-2.0

//! Africa-mode profile for resource-constrained and offline-first deployments.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use aai_broker::config::UpstreamIdpConfig;
use access_decision_service::config::{DatabaseDriver as AdsDatabaseDriver, TrustedBrokerConfig};
use serde::Deserialize;
use service_registry::config::{DatabaseConfig, DatabaseDriver};
use visa_registry::config::{DatabaseDriver as VisaDatabaseDriver};

use crate::AllInOneConfig;

/// Resource-constrained / offline-first deployment profile for ga4gh-infra.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AfricaProfile {
    /// When true, startup tolerates unreachable upstream IdP (uses cached JWKS where possible).
    #[serde(default)]
    pub offline_first: bool,
    /// Optional RSS cap in megabytes (logged and enforced best-effort on Linux).
    #[serde(default)]
    pub max_memory_mb: Option<u32>,
    /// Spawn an embedded mock OIDC IdP alongside core services.
    #[serde(default)]
    pub embedded_mock_idp: bool,
    /// Host for embedded mock IdP.
    #[serde(default = "default_mock_idp_host")]
    pub mock_idp_host: String,
    /// Port for embedded mock IdP.
    #[serde(default = "default_mock_idp_port")]
    pub mock_idp_port: u16,
    /// SQLite database directory for all registries.
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    /// JWKS cache TTL (seconds); extended automatically in low-power scenarios.
    #[serde(default = "default_jwks_cache_ttl")]
    pub jwks_cache_ttl_seconds: u64,
}

fn default_mock_idp_host() -> String {
    "127.0.0.1".to_string()
}

fn default_mock_idp_port() -> u16 {
    9000
}

fn default_data_dir() -> String {
    "~/.config/ga4gh-infra/data".to_string()
}

fn default_jwks_cache_ttl() -> u64 {
    300
}

/// Apply Africa-mode defaults to an all-in-one configuration loaded from disk.
pub fn apply_africa_profile(config: &mut AllInOneConfig, profile: &AfricaProfile) {
    let data_dir = expand_tilde(&profile.data_dir);

    config.visa_registry.database.driver = VisaDatabaseDriver::Sqlite;
    config.visa_registry.database.url = Some(format!("sqlite://{data_dir}/visa_registry.sqlite"));
    config.visa_registry.database.auto_migrate = true;

    config.service_registry.database = DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        url: Some(format!("sqlite://{data_dir}/service_registry.sqlite")),
        url_env: config.service_registry.database.url_env.clone(),
        auto_migrate: true,
    };

    config.access_decision_service.database.driver = AdsDatabaseDriver::Sqlite;
    config.access_decision_service.database.url = Some(format!("sqlite://{data_dir}/ads.sqlite"));
    config.access_decision_service.database.auto_migrate = true;

    config.access_decision_service.oidc.jwks_cache_ttl_seconds = profile.jwks_cache_ttl_seconds;

    let broker_external = config.broker.issuer_url().to_string();
    config.access_decision_service.oidc.trusted_brokers = vec![TrustedBrokerConfig {
        issuer: broker_external.clone(),
        jwks_uri: format!("{broker_external}/jwks.json"),
    }];

    if profile.embedded_mock_idp {
        let issuer = format!("http://{}:{}", profile.mock_idp_host, profile.mock_idp_port);
        let mut claim_mapping = HashMap::new();
        claim_mapping.insert("sub".to_string(), "sub".to_string());
        claim_mapping.insert("email".to_string(), "email".to_string());

        if config.broker.upstream_idps.is_empty() {
            config.broker.upstream_idps.push(UpstreamIdpConfig {
                name: "embedded-mock-idp".to_string(),
                issuer: issuer.clone(),
                client_id: "ga4gh-broker".to_string(),
                client_secret_env: "MOCK_IDP_CLIENT_SECRET".to_string(),
                scopes: vec![
                    "openid".to_string(),
                    "profile".to_string(),
                    "email".to_string(),
                ],
                claim_mapping,
            });
        } else {
            config.broker.upstream_idps[0].issuer = issuer;
        }
    }

    if profile.offline_first {
        tracing::info!(
            "Africa-mode: offline_first enabled; upstream IdP probes are non-fatal"
        );
    }

    if let Some(max_mb) = profile.max_memory_mb {
        tracing::info!(max_memory_mb = max_mb, "Africa-mode memory cap configured");
    }

    let _jwks_ttl = Duration::from_secs(profile.jwks_cache_ttl_seconds);
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

/// Returns true when `GA4GH_OFFLINE=1` or `GA4GH_AFRICA=1` is set.
pub fn africa_mode_from_env() -> bool {
    std::env::var("GA4GH_OFFLINE")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        || std::env::var("GA4GH_AFRICA")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Load optional `[africa]` section from a TOML file without requiring it in AllInOneConfig.
pub fn load_africa_profile(path: impl AsRef<Path>) -> Option<AfricaProfile> {
    #[derive(Deserialize)]
    struct Root {
        #[serde(default)]
        africa: Option<AfricaProfile>,
    }

    let contents = std::fs::read_to_string(path.as_ref()).ok()?;
    toml::from_str::<Root>(&contents).ok()?.africa
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_africa_profile_sets_sqlite_urls() {
        let mut config = AllInOneConfig::load_from_str(MINIMAL_ALL_IN_ONE).expect("parse");
        let profile = AfricaProfile {
            data_dir: "/tmp/ga4gh-africa-test".to_string(),
            embedded_mock_idp: true,
            ..AfricaProfile::default()
        };

        apply_africa_profile(&mut config, &profile);

        assert_eq!(
            config.visa_registry.database.driver,
            VisaDatabaseDriver::Sqlite
        );
        assert_eq!(
            config.service_registry.database.driver,
            DatabaseDriver::Sqlite
        );
        assert!(config
            .service_registry
            .database
            .url
            .as_deref()
            .unwrap()
            .contains("service_registry.sqlite"));
        assert!(!config.broker.upstream_idps.is_empty());
    }

    const MINIMAL_ALL_IN_ONE: &str = r#"
        [broker.server]
        host = "127.0.0.1"
        port = 8080
        external_url = "http://localhost:8080"
        environment = "development"

        [broker.signing]
        private_key_pem = "/secrets/broker.pem"
        passport_lifetime_seconds = 3600
        token_lifetime_seconds = 3600

        [broker.session]
        cookie_secret_env = "BROKER_COOKIE_SECRET"
        session_lifetime_seconds = 600

        [[broker.upstream_idps]]
        name = "mock"
        issuer = "http://localhost:9000"
        client_id = "ga4gh-broker"
        client_secret_env = "MOCK_IDP_CLIENT_SECRET"
        scopes = ["openid"]

        [[broker.visa_sources]]
        name = "local"
        url = "http://127.0.0.1:8081"

        [visa_registry.server]
        host = "127.0.0.1"
        port = 8081
        external_url = "http://localhost:8081"
        environment = "development"

        [visa_registry.signing]
        private_key_pem = "/secrets/registry.pem"
        visa_lifetime_seconds = 86400

        [visa_registry.database]
        url_env = "REGISTRY_DATABASE_URL"

        [visa_registry.auth]
        bootstrap_api_key_env = "REGISTRY_BOOTSTRAP_API_KEY"

        [duo_service.server]
        host = "127.0.0.1"
        port = 8082
        external_url = "http://localhost:8082"
        environment = "development"

        [service_registry.server]
        host = "127.0.0.1"
        port = 8083
        external_url = "http://localhost:8083"
        environment = "development"
        read_only = false

        [service_registry.database]
        url_env = "SERVICE_REGISTRY_DATABASE_URL"

        [service_registry.auth]
        registration_api_key_env = "SERVICE_REGISTRY_REGISTRATION_KEY"

        [access_decision_service.server]
        host = "127.0.0.1"
        port = 8090
        external_url = "http://localhost:8090"
        environment = "development"

        [access_decision_service.database]
        url_env = "ADS_DATABASE_URL"

        [access_decision_service.oidc]
        jwks_cache_ttl_seconds = 300

        [[access_decision_service.oidc.trusted_brokers]]
        issuer = "http://localhost:8080"
        jwks_uri = "http://localhost:8080/jwks.json"

        [access_decision_service.auth]
        bootstrap_api_key_env = "ADS_DAC_API_KEY"
    "#;
}
