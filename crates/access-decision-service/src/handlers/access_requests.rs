// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use ga4gh_types::{AccessRequest, CreateAccessRequestBody};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireResearcher;
use crate::duo::evaluate_request;
use crate::error::AdsError;
use ga4gh_types::DuoEvaluateRequest;

#[instrument(skip(state, body))]
pub async fn create_access_request(
    State(state): State<Arc<AppState>>,
    RequireResearcher(caller): RequireResearcher,
    Json(body): Json<CreateAccessRequestBody>,
) -> Result<Json<AccessRequest>, AdsError> {
    if body.researcher_id != caller.sub {
        return Err(AdsError::Forbidden);
    }

    let evaluation = evaluate_request(
        &state.store,
        &DuoEvaluateRequest {
            dataset_duo: vec![],
            dataset_id: Some(body.dataset_id),
            project_duo: vec![],
            project_id: Some(body.project_id),
            auto_approve_threshold: None,
        },
    )
    .await?;

    let request = state
        .store
        .create_access_request(&body, Some(evaluation))
        .await?;
    Ok(Json(request))
}

#[instrument(skip(state))]
pub async fn get_access_request(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireResearcher(caller): RequireResearcher,
) -> Result<Json<AccessRequest>, AdsError> {
    let request = state.store.get_access_request(id).await?;
    if request.researcher_id != caller.sub {
        return Err(AdsError::Forbidden);
    }
    Ok(Json(request))
}
