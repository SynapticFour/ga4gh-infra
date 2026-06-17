// SPDX-License-Identifier: Apache-2.0

//! Liveness probe handler.

use axum::Json;
use ga4gh_types::HealthResponse;

/// Return a simple OK health response.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}
