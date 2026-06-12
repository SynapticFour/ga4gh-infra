// SPDX-License-Identifier: Apache-2.0

//! JWKS export handler.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::Value;
use tracing::instrument;

use crate::app::AppState;

/// Return the registry signing JWKS document.
#[instrument(skip(state))]
pub async fn jwks(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(state.keys.jwks().clone())
}
