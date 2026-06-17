// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{AccessRequestListResponse, GrantListResponse, ProjectListResponse};
use tracing::instrument;

use crate::app::AppState;
use crate::auth::RequireResearcher;
use crate::error::AdsError;

#[instrument(skip(state))]
pub async fn list_my_projects(
    State(state): State<Arc<AppState>>,
    RequireResearcher(caller): RequireResearcher,
) -> Result<Json<ProjectListResponse>, AdsError> {
    let projects = state
        .store
        .list_projects_for_researcher(&caller.sub)
        .await?;
    Ok(Json(ProjectListResponse { projects }))
}

#[instrument(skip(state))]
pub async fn list_my_access_requests(
    State(state): State<Arc<AppState>>,
    RequireResearcher(caller): RequireResearcher,
) -> Result<Json<AccessRequestListResponse>, AdsError> {
    let requests = state
        .store
        .list_access_requests_for_researcher(&caller.sub)
        .await?;
    Ok(Json(AccessRequestListResponse { requests }))
}

#[instrument(skip(state))]
pub async fn list_my_grants(
    State(state): State<Arc<AppState>>,
    RequireResearcher(caller): RequireResearcher,
) -> Result<Json<GrantListResponse>, AdsError> {
    let grants = state.store.list_grants(Some(&caller.sub), None).await?;
    Ok(Json(GrantListResponse { grants }))
}
