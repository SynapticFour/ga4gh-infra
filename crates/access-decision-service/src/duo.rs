// SPDX-License-Identifier: Apache-2.0

//! DUO compatibility evaluation for datasets and research projects.
//!
//! Hierarchy matching lives in `ga4gh-types::evaluate_duo_codes` (same as agreement
//! matching). Full OWL ancestor evaluation lives in `duo-service`.

use ga4gh_types::{DuoEvaluateRequest, DuoEvaluationResult};

pub use ga4gh_types::evaluate_duo_codes;

use crate::error::AdsError;
use crate::store::AdsStore;

/// Resolve DUO codes from request body and store, then evaluate.
pub async fn evaluate_request(
    store: &AdsStore,
    request: &DuoEvaluateRequest,
) -> Result<DuoEvaluationResult, AdsError> {
    let mut dataset_duo = request.dataset_duo.clone();
    let mut project_duo = request.project_duo.clone();
    let mut threshold = request.auto_approve_threshold.unwrap_or(100);

    if let Some(dataset_id) = request.dataset_id {
        let dataset = store.get_dataset(dataset_id).await?;
        dataset_duo = dataset.duo_codes;
        if request.auto_approve_threshold.is_none() {
            threshold = dataset.auto_approve_threshold;
        }
    }

    if let Some(project_id) = request.project_id {
        let project = store.get_project(project_id).await?;
        project_duo = project.duo_codes;
    }

    Ok(evaluate_duo_codes(&dataset_duo, &project_duo, threshold))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ga4gh_types::DuoCode;

    #[test]
    fn nres_dataset_is_auto_approvable() {
        let result = evaluate_duo_codes(&[DuoCode::Nres], &[DuoCode::Gru], 100);
        assert!(result.compatible);
        assert!(result.auto_approvable);
    }

    #[test]
    fn gru_project_satisfies_gru_dataset() {
        let result = evaluate_duo_codes(&[DuoCode::Gru], &[DuoCode::Gru], 100);
        assert!(result.compatible);
        assert!(result.auto_approvable);
    }

    #[test]
    fn hmb_satisfies_gru_requirement() {
        let result = evaluate_duo_codes(&[DuoCode::Gru], &[DuoCode::Hmb], 100);
        assert!(result.compatible);
    }
}
