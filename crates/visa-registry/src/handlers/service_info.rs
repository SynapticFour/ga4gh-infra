// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Info handler.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};
use tracing::instrument;

use crate::app::AppState;

/// Return GA4GH Service Info metadata for this visa registry.
#[instrument(skip(state))]
pub async fn service_info(State(state): State<Arc<AppState>>) -> Json<ServiceInfo> {
    Json(ServiceInfo {
        id: format!(
            "{}.visa-registry",
            state.config.issuer_url().replace("https://", "")
        ),
        name: "GA4GH Visa Registry".to_string(),
        r#type: ServiceType {
            group: "org.ga4gh".to_string(),
            artifact: "visa".to_string(),
            version: "1.0".to_string(),
        },
        organization: ServiceOrganization {
            name: "GA4GH Infra".to_string(),
            url: state.config.issuer_url().to_string(),
            contact_url: None,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: Some(
            "Visa assertion store that signs GA4GH visas for researchers on behalf of DACs"
                .to_string(),
        ),
        documentation_url: None,
        created_at: None,
        updated_at: None,
        environment: Some(state.config.server.environment.clone()),
    })
}
