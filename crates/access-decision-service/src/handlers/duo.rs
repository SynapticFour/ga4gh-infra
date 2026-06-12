// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{DuoEvaluateRequest, DuoEvaluationResult};
use tracing::instrument;

use crate::app::AppState;
use crate::auth::RequireResearcher;
use crate::duo::evaluate_request;
use crate::error::AdsError;

#[instrument(skip(state, body))]
pub async fn evaluate_duo(
    State(state): State<Arc<AppState>>,
    RequireResearcher(_caller): RequireResearcher,
    Json(body): Json<DuoEvaluateRequest>,
) -> Result<Json<DuoEvaluationResult>, AdsError> {
    let result = evaluate_request(&state.store, &body).await?;
    Ok(Json(result))
}
