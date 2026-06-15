// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use ga4gh_types::{
    CreatePermissionMappingRequest, CreatePermissionSourceRequest, PermissionMapping,
    PermissionMappingListResponse, PermissionSource, PermissionSourceListResponse,
};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;

#[instrument(skip(state))]
pub async fn list_permission_sources(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
) -> Result<Json<PermissionSourceListResponse>, AdsError> {
    let sources = state.store.list_permission_sources().await?;
    Ok(Json(PermissionSourceListResponse { sources }))
}

#[instrument(skip(state))]
pub async fn list_permission_mappings(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
) -> Result<Json<PermissionMappingListResponse>, AdsError> {
    let mappings = state.store.list_permission_mappings().await?;
    Ok(Json(PermissionMappingListResponse { mappings }))
}

#[instrument(skip(state, body))]
pub async fn create_permission_source(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Json(body): Json<CreatePermissionSourceRequest>,
) -> Result<Json<PermissionSource>, AdsError> {
    let source = state.store.create_permission_source(&body).await?;
    Ok(Json(source))
}

#[instrument(skip(state, body))]
pub async fn create_permission_mapping(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Json(body): Json<CreatePermissionMappingRequest>,
) -> Result<Json<PermissionMapping>, AdsError> {
    let mapping = state.store.create_permission_mapping(&body).await?;
    Ok(Json(mapping))
}

#[instrument(skip(state))]
pub async fn delete_permission_mapping(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AdsError> {
    state.store.delete_permission_mapping(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
