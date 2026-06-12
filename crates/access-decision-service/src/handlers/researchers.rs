// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use ga4gh_types::{Researcher, ResearcherVisasResponse, SignedVisasResponse};
use tracing::instrument;

use crate::app::AppState;
use crate::auth::{RequireDac, RequireResearcher};
use crate::error::AdsError;
use crate::visas::researcher_visas;

#[instrument(skip(state))]
pub async fn get_researcher(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    RequireResearcher(caller): RequireResearcher,
) -> Result<Json<Researcher>, AdsError> {
    if caller.sub != id {
        return Err(AdsError::Forbidden);
    }
    let researcher = state.store.get_researcher(&id).await?;
    Ok(Json(researcher))
}

#[instrument(skip(state))]
pub async fn get_researcher_visas(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    RequireResearcher(caller): RequireResearcher,
) -> Result<Json<ResearcherVisasResponse>, AdsError> {
    if caller.sub != id {
        return Err(AdsError::Forbidden);
    }
    let researcher = state.store.get_researcher(&id).await?;
    let grants = state.store.list_grants(Some(&id)).await?;
    Ok(Json(researcher_visas(
        &researcher,
        &grants,
        &state.config.visas,
    )))
}

#[instrument(skip(state))]
pub async fn get_researcher_signed_visas(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    RequireDac(_operator): RequireDac,
) -> Result<Json<SignedVisasResponse>, AdsError> {
    let researcher = state.store.get_researcher(&id).await?;
    let grants = state.store.list_grants(Some(&id)).await?;
    let claims = researcher_visas(&researcher, &grants, &state.config.visas);

    let visa_jwts = if let Some(client) = &state.visa_registry {
        client.publish_and_fetch_jwts(&id, &claims.visas).await?
    } else {
        return Err(AdsError::Config(
            "visa_registry integration is not configured".to_string(),
        ));
    };

    Ok(Json(SignedVisasResponse {
        researcher_id: id,
        visa_jwts,
    }))
}
