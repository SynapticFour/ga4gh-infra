// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use chrono::Utc;
use ga4gh_types::CompatibilityCheckRequest;

use crate::app::AppState;
use crate::http_error::AgreementRegistryHttpError;

pub async fn compatibility_check(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CompatibilityCheckRequest>,
) -> Result<Json<ga4gh_types::CompatibilityCheckResult>, AgreementRegistryHttpError> {
    if body.requester_profile_id.trim().is_empty() || body.dataset_profile_id.trim().is_empty() {
        return Err(AgreementRegistryHttpError::BadRequest(
            "requester_profile_id and dataset_profile_id are required".to_string(),
        ));
    }
    let mut registry = state.registry.write().await;
    let result =
        registry.compatibility_check(&body, Utc::now(), Some("agreement-registry".into()))?;
    Ok(Json(result))
}
