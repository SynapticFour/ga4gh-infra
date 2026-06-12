// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use ga4gh_types::{AccessRequest, DacActionRequest, DacQueueResponse};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;

#[instrument(skip(state))]
pub async fn list_dac_requests(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
) -> Result<Json<DacQueueResponse>, AdsError> {
    let requests = state.store.list_dac_requests().await?;
    Ok(Json(DacQueueResponse { requests }))
}

#[instrument(skip(state, body))]
pub async fn dac_approve(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    let actor = body
        .actor
        .unwrap_or_else(|| format!("dac:{}", operator.name));
    let request = state
        .store
        .dac_approve(id, &actor, body.reason)
        .await?;
    Ok(Json(request))
}

#[instrument(skip(state, body))]
pub async fn dac_reject(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    let actor = body
        .actor
        .unwrap_or_else(|| format!("dac:{}", operator.name));
    let request = state
        .store
        .dac_reject(id, &actor, body.reason)
        .await?;
    Ok(Json(request))
}

#[instrument(skip(state, body))]
pub async fn dac_escalate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    let actor = body
        .actor
        .unwrap_or_else(|| format!("dac:{}", operator.name));
    let request = state
        .store
        .dac_escalate(id, &actor, body.reason)
        .await?;
    Ok(Json(request))
}
