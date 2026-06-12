// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use ga4gh_types::{CreateDatasetRequest, Dataset};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;

#[instrument(skip(state, body))]
pub async fn create_dataset(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Json(body): Json<CreateDatasetRequest>,
) -> Result<Json<Dataset>, AdsError> {
    if body.duo_codes.is_empty() {
        return Err(AdsError::BadRequest("duo_codes must not be empty".to_string()));
    }
    let dataset = state.store.create_dataset(&body).await?;
    Ok(Json(dataset))
}

#[instrument(skip(state))]
pub async fn get_dataset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Dataset>, AdsError> {
    let dataset = state.store.get_dataset(id).await?;
    Ok(Json(dataset))
}
