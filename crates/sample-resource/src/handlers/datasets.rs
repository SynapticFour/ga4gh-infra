// SPDX-License-Identifier: Apache-2.0

//! Protected dataset handlers using [`ExtractedPassport`].

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use ga4gh_clearinghouse::axum::ExtractedPassport;
use ga4gh_clearinghouse::PolicyCheck;
use ga4gh_types::Passport;
use serde::Serialize;
use tracing::instrument;

use crate::app::AppState;
use crate::datasets::resolve_intended_use;
use crate::duo::evaluate_duo_match;
use crate::error::SampleResourceError;

/// Metadata returned for an authorized dataset request.
#[derive(Debug, Serialize)]
pub struct DatasetResponse {
    /// Dataset identifier.
    pub id: String,
    /// Human-readable dataset name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Researcher subject from the validated Passport.
    pub subject: String,
    /// DUO codes attached to the dataset.
    pub duo: Vec<String>,
}

/// Summary response including DUO evaluation results.
#[derive(Debug, Serialize)]
pub struct DatasetSummaryResponse {
    /// Dataset metadata and access context.
    #[serde(flatten)]
    pub dataset: DatasetResponse,
    /// Whether intended use satisfies dataset DUO restrictions.
    pub duo_permitted: bool,
    /// Human-readable DUO evaluation explanation.
    pub duo_reason: String,
    /// Intended-use DUO codes used for the evaluation.
    pub intended_use: Vec<String>,
}

/// Return dataset metadata when the caller holds a controlled-access visa.
#[instrument(skip(state, passport))]
pub async fn get_dataset(
    Path(dataset_id): Path<String>,
    ExtractedPassport(passport): ExtractedPassport,
    State(state): State<Arc<AppState>>,
) -> Result<Json<DatasetResponse>, SampleResourceError> {
    let dataset = state
        .datasets
        .get(&dataset_id)
        .ok_or(SampleResourceError::NotFound)?;

    ensure_controlled_access(&state, &passport, &dataset_id).await?;

    Ok(Json(DatasetResponse {
        id: dataset.id.clone(),
        name: dataset.name.clone(),
        description: dataset.description.clone(),
        subject: passport.sub,
        duo: dataset.duo.clone(),
    }))
}

/// Return dataset metadata plus DUO policy evaluation for the caller's intended use.
#[instrument(skip(state, passport, headers))]
pub async fn get_dataset_summary(
    Path(dataset_id): Path<String>,
    headers: HeaderMap,
    ExtractedPassport(passport): ExtractedPassport,
    State(state): State<Arc<AppState>>,
) -> Result<Json<DatasetSummaryResponse>, SampleResourceError> {
    let dataset = state
        .datasets
        .get(&dataset_id)
        .ok_or(SampleResourceError::NotFound)?;

    ensure_controlled_access(&state, &passport, &dataset_id).await?;

    let intended_use = resolve_intended_use(&headers, dataset)?;
    let duo_result = evaluate_duo_match(
        &state.http_client,
        &state.config.duo_service.url,
        &dataset.duo,
        &intended_use,
    )
    .await?;

    if !duo_result.permitted {
        return Err(SampleResourceError::DuoDenied(duo_result.reason));
    }

    Ok(Json(DatasetSummaryResponse {
        dataset: DatasetResponse {
            id: dataset.id.clone(),
            name: dataset.name.clone(),
            description: dataset.description.clone(),
            subject: passport.sub,
            duo: dataset.duo.clone(),
        },
        duo_permitted: duo_result.permitted,
        duo_reason: duo_result.reason,
        intended_use,
    }))
}

async fn ensure_controlled_access(
    state: &AppState,
    passport: &Passport,
    dataset_id: &str,
) -> Result<(), SampleResourceError> {
    let visas = state.clearinghouse.extract_visas(passport).await?;
    let result = state.clearinghouse.check_policy(
        &visas,
        &PolicyCheck::HasControlledAccess {
            dataset_id: dataset_id.to_string(),
        },
    );
    if !result.permitted {
        return Err(SampleResourceError::Forbidden(result.reason));
    }
    Ok(())
}
