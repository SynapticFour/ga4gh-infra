// SPDX-License-Identifier: Apache-2.0

//! Dataset-to-intended-use DUO matching engine.

use serde::{Deserialize, Serialize};

use crate::error::DuoServiceError;
use crate::terms::{DuoCatalog, DuoCategory, DuoTerm};

/// Request body for DUO matching.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MatchRequest {
    /// DUO codes attached to a dataset or resource.
    pub dataset_duo: Vec<String>,
    /// DUO codes describing the researcher's intended use.
    pub intended_use: Vec<String>,
}

/// Result of evaluating whether intended use satisfies dataset restrictions.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MatchResponse {
    /// Whether the intended use satisfies the dataset DUO policy.
    pub permitted: bool,
    /// Human-readable explanation of the decision.
    pub reason: String,
    /// Dataset codes that were satisfied by the intended use.
    pub matched_codes: Vec<String>,
    /// Dataset codes that were not satisfied.
    pub missing_codes: Vec<String>,
}

/// Evaluate whether a researcher's intended use satisfies dataset DUO codes.
pub fn evaluate_match(
    catalog: &DuoCatalog,
    request: &MatchRequest,
) -> Result<MatchResponse, DuoServiceError> {
    if request.dataset_duo.is_empty() {
        return Err(DuoServiceError::BadRequest(
            "dataset_duo must not be empty".to_string(),
        ));
    }
    if request.intended_use.is_empty() {
        return Err(DuoServiceError::BadRequest(
            "intended_use must not be empty".to_string(),
        ));
    }

    let dataset_terms = resolve_codes(catalog, &request.dataset_duo)?;
    let intended_terms = resolve_codes(catalog, &request.intended_use)?;

    let dataset_permissions: Vec<&DuoTerm> = dataset_terms
        .iter()
        .copied()
        .filter(|term| term.category == DuoCategory::Permission)
        .collect();
    let dataset_modifiers: Vec<&DuoTerm> = dataset_terms
        .iter()
        .copied()
        .filter(|term| term.category == DuoCategory::Modifier)
        .collect();
    let intended_permissions: Vec<&DuoTerm> = intended_terms
        .iter()
        .copied()
        .filter(|term| term.category == DuoCategory::Permission)
        .collect();
    let intended_modifiers: Vec<&DuoTerm> = intended_terms
        .iter()
        .copied()
        .filter(|term| term.category == DuoCategory::Modifier)
        .collect();

    if dataset_permissions.iter().any(|term| term.code == "NRES") {
        return Ok(permitted_response(
            request,
            "dataset has no restriction (NRES)",
        ));
    }

    let mut matched_codes = Vec::new();
    let mut missing_codes = Vec::new();

    for required in dataset_permissions {
        if permission_covered(catalog, &intended_permissions, required) {
            matched_codes.push(required.code.clone());
        } else {
            missing_codes.push(required.code.clone());
        }
    }

    for required in dataset_modifiers {
        if intended_modifiers
            .iter()
            .any(|candidate| candidate.code == required.code)
        {
            matched_codes.push(required.code.clone());
        } else {
            missing_codes.push(required.code.clone());
        }
    }

    if missing_codes.is_empty() {
        Ok(MatchResponse {
            permitted: true,
            reason: "intended use satisfies all dataset DUO requirements".to_string(),
            matched_codes,
            missing_codes,
        })
    } else {
        Ok(MatchResponse {
            permitted: false,
            reason: format!(
                "intended use is missing required DUO codes: {}",
                missing_codes.join(", ")
            ),
            matched_codes,
            missing_codes,
        })
    }
}

fn resolve_codes<'a>(
    catalog: &'a DuoCatalog,
    codes: &[String],
) -> Result<Vec<&'a DuoTerm>, DuoServiceError> {
    codes.iter().map(|code| catalog.resolve(code)).collect()
}

fn permission_covered<'a>(
    catalog: &DuoCatalog,
    intended_permissions: &[&'a DuoTerm],
    required: &'a DuoTerm,
) -> bool {
    intended_permissions
        .iter()
        .any(|candidate| catalog.permission_satisfies(candidate, required))
}

fn permitted_response(request: &MatchRequest, reason: &str) -> MatchResponse {
    MatchResponse {
        permitted: true,
        reason: reason.to_string(),
        matched_codes: request.dataset_duo.clone(),
        missing_codes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terms::DuoCatalog;

    fn catalog() -> DuoCatalog {
        DuoCatalog::from_embedded().expect("catalog")
    }

    #[test]
    fn hmb_intended_use_satisfies_gru_dataset() {
        let response = evaluate_match(
            &catalog(),
            &MatchRequest {
                dataset_duo: vec!["GRU".to_string()],
                intended_use: vec!["HMB".to_string()],
            },
        )
        .expect("match");
        assert!(response.permitted);
    }

    #[test]
    fn gru_intended_use_does_not_satisfy_hmb_dataset() {
        let response = evaluate_match(
            &catalog(),
            &MatchRequest {
                dataset_duo: vec!["HMB".to_string()],
                intended_use: vec!["GRU".to_string()],
            },
        )
        .expect("match");
        assert!(!response.permitted);
        assert!(response.missing_codes.contains(&"HMB".to_string()));
    }

    #[test]
    fn requires_all_modifiers() {
        let response = evaluate_match(
            &catalog(),
            &MatchRequest {
                dataset_duo: vec!["GRU".to_string(), "NPU".to_string()],
                intended_use: vec!["GRU".to_string()],
            },
        )
        .expect("match");
        assert!(!response.permitted);
        assert!(response.missing_codes.contains(&"NPU".to_string()));
    }

    #[test]
    fn nres_dataset_permits_any_intended_use() {
        let response = evaluate_match(
            &catalog(),
            &MatchRequest {
                dataset_duo: vec!["NRES".to_string()],
                intended_use: vec!["HMB".to_string(), "NCU".to_string()],
            },
        )
        .expect("match");
        assert!(response.permitted);
    }

    #[test]
    fn rejects_empty_dataset_duo() {
        let err = evaluate_match(
            &catalog(),
            &MatchRequest {
                dataset_duo: vec![],
                intended_use: vec!["HMB".to_string()],
            },
        )
        .unwrap_err();
        assert!(matches!(err, DuoServiceError::BadRequest(_)));
    }

    #[test]
    fn rejects_empty_intended_use() {
        let err = evaluate_match(
            &catalog(),
            &MatchRequest {
                dataset_duo: vec!["GRU".to_string()],
                intended_use: vec![],
            },
        )
        .unwrap_err();
        assert!(matches!(err, DuoServiceError::BadRequest(_)));
    }

    #[test]
    fn rejects_unknown_duo_code() {
        let err = evaluate_match(
            &catalog(),
            &MatchRequest {
                dataset_duo: vec!["NOT_A_REAL_DUO_CODE".to_string()],
                intended_use: vec!["HMB".to_string()],
            },
        )
        .unwrap_err();
        assert!(matches!(err, DuoServiceError::BadRequest(_)));
    }

    #[test]
    fn loads_fixture_match_request_json() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sample_match_request.json");
        let raw = std::fs::read_to_string(path).expect("read fixture");
        let request: MatchRequest = serde_json::from_str(&raw).expect("parse fixture");
        let response = evaluate_match(&catalog(), &request).expect("match");
        assert!(response.permitted);
    }
}
