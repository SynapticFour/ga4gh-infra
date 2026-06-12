// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use ga4gh_types::{CreateProjectRequest, ResearchProject};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireResearcher;
use crate::error::AdsError;

#[instrument(skip(state, body))]
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    RequireResearcher(caller): RequireResearcher,
    Json(body): Json<CreateProjectRequest>,
) -> Result<Json<ResearchProject>, AdsError> {
    if body.researcher_id != caller.sub {
        return Err(AdsError::Forbidden);
    }
    if body.duo_codes.is_empty() {
        return Err(AdsError::BadRequest(
            "duo_codes must not be empty".to_string(),
        ));
    }
    let project = state.store.create_project(&body).await?;
    Ok(Json(project))
}

#[instrument(skip(state))]
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireResearcher(caller): RequireResearcher,
) -> Result<Json<ResearchProject>, AdsError> {
    let project = state.store.get_project(id).await?;
    if project.researcher_id != caller.sub {
        return Err(AdsError::Forbidden);
    }
    Ok(Json(project))
}
