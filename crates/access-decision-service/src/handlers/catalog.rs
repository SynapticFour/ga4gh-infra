// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use ga4gh_types::{AdsResourceType, DatasetCatalogEntry, DatasetCatalogResponse};
use serde::Deserialize;
use tracing::instrument;

use crate::app::AppState;
use crate::auth::AuthenticatedResearcher;
use crate::error::AdsError;

#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    /// Filter by resource kind (`dataset`, `compute_pool`).
    pub resource_type: Option<String>,
}

fn parse_resource_type_filter(raw: &str) -> Result<AdsResourceType, AdsError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "dataset" => Ok(AdsResourceType::Dataset),
        "compute_pool" | "compute-pool" | "computepool" => Ok(AdsResourceType::ComputePool),
        other => Err(AdsError::BadRequest(format!(
            "invalid resource_type '{other}'; expected dataset or compute_pool"
        ))),
    }
}

/// Public/institute dataset catalog — metadata safe for browsing (excludes `draft`).
#[instrument(skip(state, headers))]
pub async fn list_catalog_datasets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<CatalogQuery>,
) -> Result<Json<DatasetCatalogResponse>, AdsError> {
    let include_institute = AuthenticatedResearcher::from_headers(&state, &headers)
        .await
        .is_ok();
    let resource_type = match query.resource_type.as_deref() {
        None => None,
        Some(raw) if raw.trim().is_empty() => None,
        Some(raw) => Some(parse_resource_type_filter(raw)?),
    };
    let datasets = state
        .store
        .list_catalog_datasets(include_institute, resource_type)
        .await?;
    let entries: Vec<DatasetCatalogEntry> = datasets.iter().map(DatasetCatalogEntry::from).collect();
    Ok(Json(DatasetCatalogResponse { datasets: entries }))
}
