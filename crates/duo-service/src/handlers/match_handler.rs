// SPDX-License-Identifier: Apache-2.0

//! DUO matching handler.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use tracing::instrument;

use crate::app::AppState;
use crate::error::DuoServiceError;
use crate::matcher::{evaluate_match, MatchRequest, MatchResponse};

/// Evaluate whether intended use satisfies dataset DUO requirements.
#[instrument(skip(state, body))]
pub async fn match_duo(
    State(state): State<Arc<AppState>>,
    Json(body): Json<MatchRequest>,
) -> Result<Json<MatchResponse>, DuoServiceError> {
    let response = evaluate_match(&state.catalog, &body)?;
    tracing::info!(
        permitted = response.permitted,
        dataset_codes = ?body.dataset_duo,
        intended_codes = ?body.intended_use,
        "evaluated DUO match"
    );
    Ok(Json(response))
}
