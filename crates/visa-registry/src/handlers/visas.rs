// SPDX-License-Identifier: Apache-2.0

//! Visa assertion CRUD handlers.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use ga4gh_types::{VisaAuthority, VisaConditions, VisaType};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::error::RegistryError;
use crate::store::NewVisaAssertion;
use crate::visa::mint_visa_jwt;

/// Query parameters for listing visas for a researcher.
#[derive(Debug, Deserialize)]
pub struct ListVisasQuery {
    /// Researcher subject identifier.
    pub sub: String,
}

/// Request body for creating a visa assertion.
#[derive(Debug, Deserialize)]
pub struct CreateVisaRequest {
    /// Researcher subject identifier.
    pub sub: String,
    /// GA4GH visa type.
    pub r#type: VisaType,
    /// Assertion value.
    pub value: String,
    /// Source organization URL.
    pub source: String,
    /// Optional authority level within the source organization.
    pub by: Option<VisaAuthority>,
    /// Optional when the source organization made the assertion (Unix seconds).
    pub asserted: Option<i64>,
    /// Optional visa conditions in DNF form.
    pub conditions: Option<VisaConditions>,
    /// Optional lifetime in seconds from now for the stored assertion.
    pub expires_in_seconds: Option<u64>,
}

/// Response body for a created visa assertion.
#[derive(Debug, Serialize)]
pub struct CreateVisaResponse {
    /// Stable assertion identifier.
    pub id: Uuid,
    /// Researcher subject identifier.
    pub sub: String,
    /// GA4GH visa type.
    pub r#type: VisaType,
    /// Assertion value.
    pub value: String,
}

/// Single signed visa returned to passport assemblers.
#[derive(Debug, Serialize)]
pub struct SignedVisaRecord {
    /// Signed visa JWT string.
    pub jwt: String,
}

/// Response body for listing signed visas for a researcher.
#[derive(Debug, Serialize)]
pub struct ListVisasResponse {
    /// Signed visa JWT strings for active assertions.
    pub visas: Vec<SignedVisaRecord>,
}

/// Create a new unsigned visa assertion (DAC grant).
#[instrument(skip(state, headers, body))]
pub async fn create_visa(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateVisaRequest>,
) -> Result<(StatusCode, Json<CreateVisaResponse>), RegistryError> {
    require_api_key(&state, &headers).await?;

    if body.sub.trim().is_empty() {
        return Err(RegistryError::BadRequest(
            "sub must not be empty".to_string(),
        ));
    }
    if body.value.trim().is_empty() {
        return Err(RegistryError::BadRequest(
            "value must not be empty".to_string(),
        ));
    }
    if body.source.trim().is_empty() {
        return Err(RegistryError::BadRequest(
            "source must not be empty".to_string(),
        ));
    }

    let now = unix_now();
    let expires_at = body.expires_in_seconds.map(|seconds| now + seconds as i64);
    let assertion = state
        .store
        .create_assertion(NewVisaAssertion {
            sub: body.sub.clone(),
            visa_type: body.r#type.clone(),
            value: body.value.clone(),
            source: body.source.clone(),
            by: body.by,
            conditions: body.conditions,
            asserted: body.asserted.unwrap_or(now),
            expires_at,
        })
        .await?;

    tracing::info!(
        assertion_id = %assertion.id,
        sub = %assertion.sub,
        visa_type = %assertion.visa_type,
        "visa assertion created"
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateVisaResponse {
            id: assertion.id,
            sub: assertion.sub,
            r#type: assertion.visa_type,
            value: assertion.value,
        }),
    ))
}

/// Revoke a visa assertion by identifier.
#[instrument(skip(state, headers))]
pub async fn delete_visa(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, RegistryError> {
    require_api_key(&state, &headers).await?;
    state.store.revoke_assertion(id).await?;
    tracing::info!(assertion_id = %id, "visa assertion revoked");
    Ok(StatusCode::NO_CONTENT)
}

/// List active signed visa JWTs for a researcher subject.
#[instrument(skip(state))]
pub async fn list_visas(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListVisasQuery>,
) -> Result<Json<ListVisasResponse>, RegistryError> {
    if query.sub.trim().is_empty() {
        return Err(RegistryError::BadRequest(
            "sub query parameter is required".to_string(),
        ));
    }

    let assertions = state.store.list_active_for_sub(&query.sub).await?;
    let mut visas = Vec::with_capacity(assertions.len());
    for assertion in assertions {
        let jwt = mint_visa_jwt(&assertion, &state.config, &state.keys)?;
        visas.push(SignedVisaRecord { jwt });
    }

    Ok(Json(ListVisasResponse { visas }))
}

async fn require_api_key(state: &AppState, headers: &HeaderMap) -> Result<(), RegistryError> {
    let api_key = headers
        .get("X-API-Key")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|value| value.to_str().ok())
        .ok_or(RegistryError::Unauthorized)?;
    state.store.verify_api_key(api_key).await
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_request_deserializes() {
        let body = serde_json::json!({
            "sub": "researcher@example.org",
            "type": "ControlledAccessGrants",
            "value": "dataset-abc",
            "source": "https://dac.example.org",
            "by": "dac"
        });
        let request: CreateVisaRequest = serde_json::from_value(body).expect("deserialize");
        assert_eq!(request.sub, "researcher@example.org");
        assert_eq!(request.r#type, VisaType::ControlledAccessGrants);
    }
}
