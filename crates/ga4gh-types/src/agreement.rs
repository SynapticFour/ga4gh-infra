// SPDX-License-Identifier: Apache-2.0

//! Policy profiles, agreement templates, and compatibility check types.
//!
//! Operationalizes GA4GH Machine Readable Consent Guidance (MRCG) at the level of
//! institution/dataset DUO code sets — not legal document text.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::duo::DuoCode;
use crate::visa::VisaType;

/// One institution's or dataset's data-use policy expressed as annotated DUO codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyProfile {
    /// Stable profile identifier (e.g. `profile.ega.dataset-123`).
    pub id: String,
    /// Owning institution, dataset, or researcher cohort identifier.
    pub owner: String,
    /// DUO permission and modifier assertions for this policy.
    pub duo_codes: Vec<DuoCodeAssertion>,
    /// Optional agreement template this profile was derived from.
    pub based_on_template: Option<String>,
    /// Profile version string (semver or institutional scheme).
    pub version: String,
    /// When this policy profile became effective.
    pub effective_date: DateTime<Utc>,
    /// Opaque pointer to legal/consent source (e.g. Ferrum DTA record id) — not resolved here.
    pub source_document_ref: Option<String>,
    /// Visa types the requester is asserted to hold (when known); used for template visa checks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visa_types: Vec<VisaType>,
}

/// A single DUO code with optional parameter value and audit rationale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuoCodeAssertion {
    /// DUO permission or modifier code.
    pub code: DuoCode,
    /// Parameter value for codes that require one (e.g. MONDO id for `DS`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier_value: Option<String>,
    /// Human-readable mapping rationale (MRCG worksheet / DAC audit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// Named, versioned DUO pattern for a known agreement type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgreementTemplate {
    /// Stable template id (e.g. `ega-general-research-use-v1`).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Template version.
    pub version: String,
    /// Short description of the agreement pattern.
    pub description: String,
    /// DUO codes that must be satisfied by the requester.
    pub required_duo_codes: Vec<DuoCode>,
    /// Optional additional DUO codes permitted on top of required set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_duo_codes: Vec<DuoCode>,
    /// Visa types expected on the requester side for full automation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_visa_types: Vec<VisaType>,
    /// Link to human-readable agreement or policy text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_url: Option<String>,
    /// When true, template is documented as illustrative — not a verbatim published agreement.
    #[serde(default)]
    pub is_illustrative: bool,
}

/// Input to a compatibility check between requester and dataset policy profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityCheckRequest {
    /// Registered requester (researcher-side) policy profile id.
    pub requester_profile_id: String,
    /// Registered dataset-side policy profile id.
    pub dataset_profile_id: String,
}

/// Outcome of comparing requester and dataset policy profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityCheckResult {
    /// Whether DUO-level technical compatibility is satisfied.
    pub compatible: bool,
    /// Template id when the check aligned with a known agreement template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_template: Option<String>,
    /// Dataset/requester DUO codes that were satisfied.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub satisfied_codes: Vec<DuoCode>,
    /// DUO codes required by the dataset but not satisfied by the requester.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsatisfied_codes: Vec<DuoCode>,
    /// Human-readable conditions DUO cannot fully automate (DAC discretion, legal review, etc.).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    /// Identifier for DACReS-style audit storage (assigned by agreement-registry).
    pub decision_record_id: String,
}

/// Audit record for a single compatibility evaluation (DACReS-oriented).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Same id as `CompatibilityCheckResult.decision_record_id`.
    pub id: String,
    /// Requester profile id evaluated.
    pub requester_profile_id: String,
    /// Dataset profile id evaluated.
    pub dataset_profile_id: String,
    /// Full compatibility outcome.
    pub result: CompatibilityCheckResult,
    /// When the check was performed.
    pub checked_at: DateTime<Utc>,
    /// Optional actor (service account, DAC member id) — opaque string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_by: Option<String>,
}

/// Response body for listing agreement templates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgreementTemplateListResponse {
    /// Registered agreement templates.
    pub templates: Vec<AgreementTemplate>,
}

/// Response body for listing compatibility decision records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecordListResponse {
    /// Stored decision records.
    pub decisions: Vec<DecisionRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile(id: &str, codes: Vec<DuoCode>) -> PolicyProfile {
        PolicyProfile {
            id: id.to_string(),
            owner: "owner.example.org".to_string(),
            duo_codes: codes
                .into_iter()
                .map(|code| DuoCodeAssertion {
                    code,
                    modifier_value: None,
                    rationale: None,
                })
                .collect(),
            based_on_template: None,
            version: "1.0.0".to_string(),
            effective_date: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                .expect("date")
                .with_timezone(&Utc),
            source_document_ref: None,
            visa_types: Vec::new(),
        }
    }

    #[test]
    fn policy_profile_round_trip() {
        let profile = sample_profile("profile.test", vec![DuoCode::Gru, DuoCode::Npu]);
        let json = serde_json::to_string(&profile).expect("serialize");
        let decoded: PolicyProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(profile, decoded);
    }

    #[test]
    fn agreement_template_omits_empty_optional_fields() {
        let template = AgreementTemplate {
            id: "test-v1".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "desc".to_string(),
            required_duo_codes: vec![DuoCode::Gru],
            allowed_duo_codes: vec![],
            required_visa_types: vec![],
            reference_url: None,
            is_illustrative: true,
        };
        let json = serde_json::to_value(&template).expect("serialize");
        assert!(json.get("allowed_duo_codes").is_none());
        assert!(json.get("is_illustrative").is_some());
    }

    #[test]
    fn duo_code_assertion_with_modifier_round_trip() {
        let assertion = DuoCodeAssertion {
            code: DuoCode::Ds,
            modifier_value: Some("MONDO:0011429".to_string()),
            rationale: Some("juvenile idiopathic arthritis".to_string()),
        };
        let json = serde_json::to_string(&assertion).expect("serialize");
        let decoded: DuoCodeAssertion = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(assertion, decoded);
    }

    #[test]
    fn compatibility_result_round_trip() {
        let result = CompatibilityCheckResult {
            compatible: false,
            matched_template: None,
            satisfied_codes: vec![DuoCode::Gru],
            unsatisfied_codes: vec![DuoCode::Npu],
            conditions: vec!["requires DAC sign-off".to_string()],
            decision_record_id: "decision-001".to_string(),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let decoded: CompatibilityCheckResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result, decoded);
    }
}
