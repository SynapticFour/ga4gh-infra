// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use ga4gh_types::DecisionRecordListResponse;
use serde::Deserialize;

use crate::app::AppState;
use crate::http_error::AgreementRegistryHttpError;

#[derive(Debug, Deserialize)]
pub struct DecisionListQuery {
    pub profile_id: Option<String>,
}

pub async fn list_decisions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DecisionListQuery>,
) -> Result<Json<DecisionRecordListResponse>, AgreementRegistryHttpError> {
    let registry = state.registry.read().await;
    let decisions = registry
        .list_decisions(query.profile_id.as_deref())
        .into_iter()
        .cloned()
        .collect();
    Ok(Json(DecisionRecordListResponse { decisions }))
}
