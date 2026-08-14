use std::sync::Arc;
use std::time::Duration;

use ga4gh_clearinghouse::{JwksCache, TrustedBroker};

use crate::clients::UpstreamClients;
use crate::config::AdminUiConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AdminUiConfig>,
    pub clients: UpstreamClients,
    pub jwks: Arc<JwksCache>,
}

impl AppState {
    pub fn new(config: AdminUiConfig) -> anyhow::Result<Self> {
        let issuer = config.broker_public_url().trim_end_matches('/').to_string();
        let jwks_uri = format!("{}/jwks.json", config.broker_base_url.trim_end_matches('/'));
        let jwks = JwksCache::new(
            vec![TrustedBroker::new(issuer, jwks_uri)],
            Duration::from_secs(300),
        )
        .map_err(|err| anyhow::anyhow!("broker JWKS cache: {err}"))?;
        let config = Arc::new(config);
        let clients = UpstreamClients::new(Arc::clone(&config));
        Ok(Self {
            config,
            clients,
            jwks: Arc::new(jwks),
        })
    }
}
