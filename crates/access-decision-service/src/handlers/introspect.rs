// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use ga4gh_types::{IntrospectRequest, IntrospectResponse, PassportClaims};
use tracing::instrument;

use crate::app::AppState;
use crate::auth::RequireServiceAuth;
use crate::error::AdsError;

#[instrument(skip(state, body, auth))]
pub async fn introspect(
    State(state): State<Arc<AppState>>,
    auth: RequireServiceAuth,
    Json(body): Json<IntrospectRequest>,
) -> Result<Json<IntrospectResponse>, AdsError> {
    let passport: PassportClaims = state
        .jwks
        .verify_and_decode(&body.token)
        .await
        .map_err(AdsError::from)?;

    if let Some(caller_sub) = &auth.sub {
        if caller_sub != &passport.sub {
            return Err(AdsError::Forbidden);
        }
    }

    let grants = state
        .store
        .active_grants_for_resource(&passport.sub, body.dataset_id, &body.resource)
        .await?;

    if grants.is_empty() {
        return Ok(Json(IntrospectResponse {
            active: false,
            sub: Some(passport.sub),
            grant_ids: vec![],
            duo_codes: vec![],
            exp: Some(passport.exp),
            reason: Some("no active grant for resource".to_string()),
        }));
    }

    let grant_ids: Vec<_> = grants.iter().map(|g| g.id).collect();
    let mut duo_codes = grants.first().map(|g| g.duo_codes.clone()).unwrap_or_default();
    for grant in &grants[1..] {
        for code in &grant.duo_codes {
            if !duo_codes.contains(code) {
                duo_codes.push(*code);
            }
        }
    }

    Ok(Json(IntrospectResponse {
        active: true,
        sub: Some(passport.sub),
        grant_ids,
        duo_codes,
        exp: Some(passport.exp),
        reason: None,
    }))
}
