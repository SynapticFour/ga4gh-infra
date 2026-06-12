// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use ga4gh_types::{Grant, GrantListResponse};
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{AuthenticatedResearcher, DacOperator};
use crate::error::AdsError;

#[derive(Debug, Deserialize)]
pub struct GrantListQuery {
    pub researcher_id: Option<String>,
}

enum GrantAuth {
    Researcher(AuthenticatedResearcher),
    Dac,
}

async fn authorize_grants(state: &AppState, headers: &HeaderMap) -> Result<GrantAuth, AdsError> {
    if DacOperator::from_headers(state, headers).await.is_ok() {
        return Ok(GrantAuth::Dac);
    }
    let researcher = AuthenticatedResearcher::from_headers(state, headers).await?;
    Ok(GrantAuth::Researcher(researcher))
}

#[instrument(skip(state, headers))]
pub async fn list_grants(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<GrantListQuery>,
) -> Result<Json<GrantListResponse>, AdsError> {
    let grants = match authorize_grants(&state, &headers).await? {
        GrantAuth::Researcher(researcher) => state.store.list_grants(Some(&researcher.sub)).await?,
        GrantAuth::Dac => {
            state
                .store
                .list_grants(query.researcher_id.as_deref())
                .await?
        }
    };
    Ok(Json(GrantListResponse { grants }))
}

#[instrument(skip(state, headers))]
pub async fn get_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Grant>, AdsError> {
    let grant = state.store.get_grant(id).await?;
    match authorize_grants(&state, &headers).await? {
        GrantAuth::Researcher(researcher) if grant.researcher_id == researcher.sub => {}
        GrantAuth::Dac => {}
        _ => return Err(AdsError::Forbidden),
    }
    Ok(Json(grant))
}

#[instrument(skip(state, headers))]
pub async fn revoke_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Grant>, AdsError> {
    DacOperator::from_headers(&state, &headers).await?;
    let grant = state.store.revoke_grant(id).await?;
    Ok(Json(grant))
}
