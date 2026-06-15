// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use ga4gh_types::{AccessRequest, DacActionRequest, DacQueueResponse};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;
use crate::query::DacGroupQuery;

fn require_reason(reason: &Option<String>, action: &str) -> Result<(), AdsError> {
    if reason.as_ref().is_some_and(|s| !s.trim().is_empty()) {
        Ok(())
    } else {
        Err(AdsError::BadRequest(format!(
            "reason is required for {action}"
        )))
    }
}

#[instrument(skip(state))]
pub async fn list_dac_requests(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Query(filter): Query<DacGroupQuery>,
) -> Result<Json<DacQueueResponse>, AdsError> {
    let requests = state.store.list_dac_requests(filter.filter()).await?;
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
    let request = state.store.dac_approve(id, &actor, body.reason).await?;
    Ok(Json(request))
}

#[instrument(skip(state, body))]
pub async fn dac_reject(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    require_reason(&body.reason, "reject")?;
    let actor = body
        .actor
        .unwrap_or_else(|| format!("dac:{}", operator.name));
    let request = state.store.dac_reject(id, &actor, body.reason).await?;
    Ok(Json(request))
}

#[instrument(skip(state, body))]
pub async fn dac_escalate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    require_reason(&body.reason, "escalate")?;
    let actor = body
        .actor
        .unwrap_or_else(|| format!("dac:{}", operator.name));
    let request = state.store.dac_escalate(id, &actor, body.reason).await?;
    Ok(Json(request))
}
