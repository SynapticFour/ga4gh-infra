// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{
    CreatePermissionMappingRequest, CreatePermissionSourceRequest, PermissionMapping,
    PermissionSource,
};
use tracing::instrument;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;

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
