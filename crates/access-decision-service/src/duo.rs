// SPDX-License-Identifier: Apache-2.0

//! DUO compatibility evaluation for datasets and research projects.

use ga4gh_types::{is_permission, permission_satisfies, DuoCode, DuoEvaluateRequest, DuoEvaluationResult};

use crate::error::AdsError;
use crate::store::AdsStore;

/// Evaluate DUO compatibility between a dataset and research project.
pub fn evaluate_duo_codes(
    dataset_duo: &[DuoCode],
    project_duo: &[DuoCode],
    threshold: u8,
) -> DuoEvaluationResult {
    if dataset_duo.is_empty() {
        return DuoEvaluationResult {
            compatible: false,
            score: 0,
            auto_approvable: false,
            reason: "dataset has no DUO codes".to_string(),
            matched_codes: vec![],
            missing_codes: vec![],
            procedural_modifiers: vec![],
        };
    }

    if project_duo.is_empty() {
        return DuoEvaluationResult {
            compatible: false,
            score: 0,
            auto_approvable: false,
            reason: "project has no intended-use DUO codes".to_string(),
            matched_codes: vec![],
            missing_codes: dataset_duo.to_vec(),
            procedural_modifiers: vec![],
        };
    }

    if dataset_duo.contains(&DuoCode::Nres) {
        return DuoEvaluationResult {
            compatible: true,
            score: 100,
            auto_approvable: threshold <= 100,
            reason: "dataset has no restriction (NRES)".to_string(),
            matched_codes: vec![DuoCode::Nres],
            missing_codes: vec![],
            procedural_modifiers: vec![],
        };
    }

    let mut matched_codes = Vec::new();
    let mut missing_codes = Vec::new();
    let mut procedural_modifiers = Vec::new();

    for required in dataset_duo {
        if is_permission(*required) {
            let satisfied = project_duo
                .iter()
                .any(|requester| permission_satisfies(*requester, *required));
            if satisfied {
                matched_codes.push(*required);
            } else {
                missing_codes.push(*required);
            }
        } else if project_duo.contains(required) {
            matched_codes.push(*required);
        } else {
            missing_codes.push(*required);
            procedural_modifiers.push(format!("missing modifier {required}"));
        }
    }

    let permission_total = dataset_duo
        .iter()
        .filter(|code| is_permission(**code))
        .count()
        .max(1);
    let permission_matched = matched_codes
        .iter()
        .filter(|code| is_permission(**code))
        .count();
    let score = ((permission_matched as f32 / permission_total as f32) * 100.0).round() as u8;

    let compatible = missing_codes.is_empty();
    let auto_approvable = compatible && score >= threshold && procedural_modifiers.is_empty();
    let reason = if compatible {
        "project intended use satisfies dataset DUO policy".to_string()
    } else {
        format!("unsatisfied DUO codes: {missing_codes:?}")
    };

    DuoEvaluationResult {
        compatible,
        score,
        auto_approvable,
        reason,
        matched_codes,
        missing_codes,
        procedural_modifiers,
    }
}

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

    Ok(evaluate_duo_codes(
        &dataset_duo,
        &project_duo,
        threshold,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
