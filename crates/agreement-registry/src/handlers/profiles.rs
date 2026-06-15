// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use ga4gh_types::PolicyProfile;

use crate::app::AppState;
use crate::http_error::AgreementRegistryHttpError;

pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PolicyProfile>, AgreementRegistryHttpError> {
    let registry = state.registry.read().await;
    let profile = registry.get_profile(&id)?.clone();
    Ok(Json(profile))
}

pub async fn register_profile(
    State(state): State<Arc<AppState>>,
    Json(profile): Json<PolicyProfile>,
) -> Result<Json<PolicyProfile>, AgreementRegistryHttpError> {
    if profile.id.trim().is_empty() {
        return Err(AgreementRegistryHttpError::BadRequest(
            "profile id is required".to_string(),
        ));
    }
    let mut registry = state.registry.write().await;
    registry.register_profile(profile.clone());
    Ok(Json(profile))
}
