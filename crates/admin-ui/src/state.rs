use std::sync::Arc;

use crate::clients::UpstreamClients;
use crate::config::AdminUiConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AdminUiConfig>,
    pub clients: UpstreamClients,
}

impl AppState {
    pub fn new(config: AdminUiConfig) -> Self {
        let config = Arc::new(config);
        let clients = UpstreamClients::new(Arc::clone(&config));
        Self { config, clients }
    }
}
