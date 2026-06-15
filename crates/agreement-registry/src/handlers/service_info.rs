// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};

use crate::app::AppState;

pub async fn service_info(State(state): State<Arc<AppState>>) -> Json<ServiceInfo> {
    Json(ServiceInfo {
        id: format!(
            "{}.agreement-registry",
            state.config.external_url().replace("https://", "")
        ),
        name: "GA4GH Agreement Registry".to_string(),
        r#type: ServiceType {
            group: "org.ga4gh".to_string(),
            artifact: "agreement-registry".to_string(),
            version: "0.1".to_string(),
        },
        organization: ServiceOrganization {
            name: "GA4GH Infra".to_string(),
            url: state.config.external_url().to_string(),
            contact_url: None,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: Some(
            "Policy profile and agreement template registry with DUO compatibility checks"
                .to_string(),
        ),
        documentation_url: None,
        created_at: None,
        updated_at: None,
        environment: Some(state.config.server.environment.clone()),
    })
}
