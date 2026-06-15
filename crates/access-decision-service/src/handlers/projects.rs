// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use ga4gh_types::{CreateProjectRequest, ProjectListResponse, ResearchProject};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{AuthenticatedResearcher, DacOperator, RequireDac};
use crate::error::AdsError;

#[instrument(skip(state, body, headers))]
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateProjectRequest>,
) -> Result<Json<ResearchProject>, AdsError> {
    if DacOperator::from_headers(&state, &headers).await.is_err() {
        let caller = AuthenticatedResearcher::from_headers(&state, &headers).await?;
        if body.researcher_id != caller.sub {
            return Err(AdsError::Forbidden);
        }
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
pub async fn list_projects(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
) -> Result<Json<ProjectListResponse>, AdsError> {
    let projects = state.store.list_projects().await?;
    Ok(Json(ProjectListResponse { projects }))
}

#[instrument(skip(state, headers))]
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ResearchProject>, AdsError> {
    let project = state.store.get_project(id).await?;
    if DacOperator::from_headers(&state, &headers).await.is_ok() {
        return Ok(Json(project));
    }
    let researcher = AuthenticatedResearcher::from_headers(&state, &headers).await?;
    if project.researcher_id != researcher.sub {
        return Err(AdsError::Forbidden);
    }
    Ok(Json(project))
}
