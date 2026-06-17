// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use ga4gh_types::{DatasetCatalogEntry, DatasetCatalogResponse};
use tracing::instrument;

use crate::app::AppState;
use crate::auth::AuthenticatedResearcher;
use crate::error::AdsError;

/// Public/institute dataset catalog — metadata safe for browsing (excludes `draft`).
#[instrument(skip(state, headers))]
pub async fn list_catalog_datasets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<DatasetCatalogResponse>, AdsError> {
    let include_institute = AuthenticatedResearcher::from_headers(&state, &headers)
        .await
        .is_ok();
    let datasets = state
        .store
        .list_catalog_datasets(include_institute)
        .await?;
    let entries: Vec<DatasetCatalogEntry> = datasets.iter().map(DatasetCatalogEntry::from).collect();
    Ok(Json(DatasetCatalogResponse { datasets: entries }))
}
