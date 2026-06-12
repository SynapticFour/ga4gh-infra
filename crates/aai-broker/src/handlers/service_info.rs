// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Info handler.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};
use tracing::instrument;

use crate::app::AppState;

/// Return GA4GH Service Info metadata for this broker.
#[instrument(skip(state))]
pub async fn service_info(State(state): State<Arc<AppState>>) -> Json<ServiceInfo> {
    Json(ServiceInfo {
        id: format!("{}.aai-broker", state.config.issuer_url().replace("https://", "")),
        name: "GA4GH AAI Broker".to_string(),
        r#type: ServiceType {
            group: "org.ga4gh".to_string(),
            artifact: "passport".to_string(),
            version: "1.2".to_string(),
        },
        organization: ServiceOrganization {
            name: "GA4GH Infra".to_string(),
            url: state.config.issuer_url().to_string(),
            contact_url: None,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: Some(
            "OIDC Relying Party broker that mints GA4GH Passports after upstream IdP authentication"
                .to_string(),
        ),
        documentation_url: None,
        created_at: None,
        updated_at: None,
        environment: Some(state.config.server.environment.clone()),
    })
}
