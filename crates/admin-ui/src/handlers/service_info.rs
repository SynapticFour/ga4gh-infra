// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Info for admin-ui (used by e2e health waits and service registry).

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{ServiceInfo, ServiceOrganization, ServiceType};

use crate::state::AppState;

/// Return GA4GH Service Info metadata for the admin dashboard.
pub async fn service_info(State(state): State<Arc<AppState>>) -> Json<ServiceInfo> {
    let base = state.config.public_base_url.trim_end_matches('/');
    Json(ServiceInfo {
        id: format!(
            "{}.admin-ui",
            base.replace("https://", "").replace("http://", "")
        ),
        name: "GA4GH Admin UI".to_string(),
        r#type: ServiceType {
            group: "org.ga4gh".to_string(),
            artifact: "admin-ui".to_string(),
            version: "1.0".to_string(),
        },
        organization: ServiceOrganization {
            name: "GA4GH Infra".to_string(),
            url: base.to_string(),
            contact_url: None,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: Some(
            "Operator dashboard for DAC review, datasets, and service registry".into(),
        ),
        documentation_url: None,
        created_at: None,
        updated_at: None,
        environment: None,
    })
}
