// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use ga4gh_types::AuditEventListResponse;
use serde::Deserialize;
use tracing::instrument;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;
use crate::query::DacGroupQuery;

#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(flatten)]
    pub dac_group: DacGroupQuery,
}

fn default_limit() -> u32 {
    50
}

#[instrument(skip(state))]
pub async fn list_audit_events(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Query(query): Query<AuditListQuery>,
) -> Result<Json<AuditEventListResponse>, AdsError> {
    let limit = query.limit.clamp(1, 500);
    let events = state
        .store
        .list_audit_events(limit, query.dac_group.filter())
        .await?;
    Ok(Json(AuditEventListResponse { events }))
}
