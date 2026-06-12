// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};
use tracing::instrument;

use crate::app::AppState;

#[instrument(skip(state))]
pub async fn service_info(State(state): State<Arc<AppState>>) -> Json<ServiceInfo> {
    Json(ServiceInfo {
        id: format!(
            "{}.access-decision-service",
            state.config.external_url().replace("https://", "")
        ),
        name: "GA4GH Access Decision Service".to_string(),
        r#type: ServiceType {
            group: "org.ga4gh".to_string(),
            artifact: "access-decision-service".to_string(),
            version: "1.0.0".to_string(),
        },
        organization: ServiceOrganization {
            name: "GA4GH Infra".to_string(),
            url: state.config.external_url().to_string(),
            contact_url: None,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: Some(
            "Access requests, DUO evaluation, DAC workflows, grants, and authorization introspection"
                .to_string(),
        ),
        documentation_url: None,
        created_at: None,
        updated_at: None,
        environment: Some(state.config.server.environment.clone()),
    })
}
