// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{Grant, ResearcherSyncRequest};
use tracing::instrument;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;
use crate::permissions::sync_researcher;

/// Sync researcher profile and apply institutional permission mappings (broker hook).
#[instrument(skip(state, body))]
pub async fn sync_researcher_handler(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Json(body): Json<ResearcherSyncRequest>,
) -> Result<Json<Vec<Grant>>, AdsError> {
    if body.sub.trim().is_empty() {
        return Err(AdsError::BadRequest("sub must not be empty".to_string()));
    }
    let grants = sync_researcher(&state.store, &body).await?;
    Ok(Json(grants))
}
