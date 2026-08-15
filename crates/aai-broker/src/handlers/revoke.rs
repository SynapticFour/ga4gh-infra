// SPDX-License-Identifier: Apache-2.0

//! Public Passport revocation list and authenticated emergency revoke.

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use tracing::instrument;

use crate::app::AppState;
use crate::error::BrokerError;

#[derive(Debug, Deserialize)]
pub struct RevokePassportsRequest {
    pub sub: Option<String>,
    pub jti: Option<String>,
    pub visa_jti: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct RevokedPassportsResponse {
    pub jtis: Vec<String>,
}

/// Public list consumed by clearinghouses (`GET /revoked-passports`).
pub async fn list_revoked_passports(
    State(state): State<Arc<AppState>>,
) -> Json<RevokedPassportsResponse> {
    Json(RevokedPassportsResponse {
        jtis: state.passport_ledger.revoked_jtis().await,
    })
}

/// Emergency revoke by subject, Passport `jti`, or embedded visa `jti`.
#[instrument(skip(state, headers, body))]
pub async fn revoke_passports(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RevokePassportsRequest>,
) -> Result<Json<RevokedPassportsResponse>, BrokerError> {
    let presented = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .ok_or(BrokerError::Unauthorized)?;
    let expected = state
        .config
        .admin_api_key()
        .map_err(|_| BrokerError::Unauthorized)?;
    if expected.is_empty()
        || !ga4gh_http::constant_time_eq(presented.as_bytes(), expected.as_bytes())
    {
        return Err(BrokerError::Unauthorized);
    }
    if body.sub.is_none() && body.jti.is_none() && body.visa_jti.is_none() {
        return Err(BrokerError::BadRequest(
            "revoke-passports requires sub, jti, or visa_jti".to_string(),
        ));
    }
    let added = state
        .passport_ledger
        .revoke(
            body.sub.as_deref(),
            body.jti.as_deref(),
            body.visa_jti.as_deref(),
        )
        .await?;
    tracing::info!(
        audit = true,
        event = "passport.revoked",
        sub = ?body.sub,
        jti = ?body.jti,
        visa_jti = ?body.visa_jti,
        added,
        "passports revoked"
    );
    Ok(Json(RevokedPassportsResponse {
        jtis: state.passport_ledger.revoked_jtis().await,
    }))
}
