// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use ga4gh_types::{AgreementTemplate, AgreementTemplateListResponse};

use crate::app::AppState;
use crate::http_error::AgreementRegistryHttpError;

pub async fn list_templates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AgreementTemplateListResponse>, AgreementRegistryHttpError> {
    let registry = state.registry.read().await;
    let templates = registry.list_templates().into_iter().cloned().collect();
    Ok(Json(AgreementTemplateListResponse { templates }))
}

pub async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AgreementTemplate>, AgreementRegistryHttpError> {
    let registry = state.registry.read().await;
    let template = registry.get_template(&id)?.clone();
    Ok(Json(template))
}
