// SPDX-License-Identifier: Apache-2.0

//! DUO policy-profile compatibility matching (levels 2–3 of the agreement model).
//!
//! This is a **technical** check only. Legal review, jurisdictional regulation, and
//! full DAC workflow remain out of scope — see `conditions` on results.

use std::collections::HashSet;

use uuid::Uuid;

use crate::agreement::{
    AgreementTemplate, CompatibilityCheckResult, DuoCodeAssertion, PolicyProfile,
};
use crate::duo::DuoCode;

/// Returns `true` when `code` is a DUO permission (vs modifier).
pub fn is_permission(code: DuoCode) -> bool {
    code.is_permission()
}

/// Returns `true` when requester permission satisfies dataset requirement in DUO hierarchy.
///
/// Mirrors the direction used in `duo-service`: a more specific requester permission
/// (e.g. HMB) satisfies a broader dataset requirement (e.g. GRU).
pub fn permission_satisfies(requester: DuoCode, required: DuoCode) -> bool {
    if required == DuoCode::Nres {
        return true;
    }
    if requester == required {
        return true;
    }
    permission_ancestors(requester).contains(&required)
}

fn permission_ancestors(code: DuoCode) -> HashSet<DuoCode> {
    // Minimal OWL-aligned edges for common GA4GH archive permissions.
    // Broader terms appear as ancestors of narrower requester permissions.
    let mut set = HashSet::new();
    set.insert(code);
    match code {
        DuoCode::Hmb | DuoCode::Ds | DuoCode::Poa | DuoCode::Gso => {
            set.insert(DuoCode::Gru);
        }
        DuoCode::GruCc => {
            set.insert(DuoCode::Gru);
            set.insert(DuoCode::Cc);
        }
        _ => {}
    }
    set
}

/// Modifiers that imply human DAC / procedural steps even when DUO codes match.
pub fn procedural_modifier(code: DuoCode) -> Option<&'static str> {
    match code {
        DuoCode::Irb => Some("ethics approval (IRB) must be confirmed by DAC"),
        DuoCode::Col => Some("collaboration requirement must be confirmed by DAC"),
        DuoCode::Pub => Some("publication requirement must be acknowledged in access agreement"),
        DuoCode::Mor => Some("publication moratorium dates must be verified by DAC"),
        DuoCode::Gs => Some("geographic restriction must be verified against researcher location"),
        DuoCode::Us | DuoCode::Ps | DuoCode::Is => {
            Some("user/project/institution-specific restriction requires DAC review")
        }
        _ => None,
    }
}

/// Evaluate technical compatibility between requester and dataset profiles.
pub fn check_compatibility(
    requester: &PolicyProfile,
    dataset: &PolicyProfile,
    template: Option<&AgreementTemplate>,
) -> CompatibilityCheckResult {
    let decision_record_id = format!("decision-{}", Uuid::new_v4());

    let dataset_permissions: Vec<DuoCode> = dataset
        .duo_codes
        .iter()
        .filter(|a| is_permission(a.code))
        .map(|a| a.code)
        .collect();
    let dataset_modifiers: Vec<&DuoCodeAssertion> = dataset
        .duo_codes
        .iter()
        .filter(|a| !is_permission(a.code))
        .collect();

    let requester_permissions: Vec<DuoCode> = requester
        .duo_codes
        .iter()
        .filter(|a| is_permission(a.code))
        .map(|a| a.code)
        .collect();
    let requester_modifiers: HashSet<DuoCode> = requester
        .duo_codes
        .iter()
        .filter(|a| !is_permission(a.code))
        .map(|a| a.code)
        .collect();

    let mut satisfied_codes = Vec::new();
    let mut unsatisfied_codes = Vec::new();
    let mut conditions = Vec::new();

    for assertion in &dataset.duo_codes {
        if assertion.code == DuoCode::Ds && assertion.modifier_value.is_none() {
            conditions.push(
                "disease-specific (DS) dataset requires MONDO or equivalent modifier_value"
                    .to_string(),
            );
        }
    }

    if dataset_permissions.contains(&DuoCode::Nres) {
        satisfied_codes.extend(dataset_permissions.clone());
    } else {
        for required in &dataset_permissions {
            if requester_permissions
                .iter()
                .any(|r| permission_satisfies(*r, *required))
            {
                satisfied_codes.push(*required);
            } else {
                unsatisfied_codes.push(*required);
            }
        }
    }

    for modifier in &dataset_modifiers {
        if requester_modifiers.contains(&modifier.code) {
            satisfied_codes.push(modifier.code);
        } else {
            unsatisfied_codes.push(modifier.code);
        }
        if let Some(note) = procedural_modifier(modifier.code) {
            conditions.push(note.to_string());
        }
    }

    for assertion in &requester.duo_codes {
        if let Some(note) = procedural_modifier(assertion.code) {
            if !conditions.iter().any(|c| c == note) {
                conditions.push(note.to_string());
            }
        }
    }

    let matched_template = template
        .map(|t| t.id.clone())
        .or_else(|| dataset.based_on_template.clone());

    if let Some(tmpl) = template {
        for required in &tmpl.required_duo_codes {
            if is_permission(*required) {
                if !requester_permissions
                    .iter()
                    .any(|r| permission_satisfies(*r, *required))
                    && !unsatisfied_codes.contains(required)
                {
                    unsatisfied_codes.push(*required);
                }
            } else if !requester_modifiers.contains(required)
                && !unsatisfied_codes.contains(required)
            {
                unsatisfied_codes.push(*required);
            }
        }
        for visa in &tmpl.required_visa_types {
            if !requester.visa_types.contains(visa) {
                conditions.push(format!(
                    "template requires visa type {:?} — not asserted on requester profile",
                    visa
                ));
            }
        }
        if tmpl.is_illustrative {
            conditions.push(
                "matched template is illustrative; institutional legal review still required"
                    .to_string(),
            );
        }
    }

    if dataset.source_document_ref.is_some() {
        conditions.push(
            "dataset policy references external legal text (source_document_ref) — not verified here"
                .to_string(),
        );
    }

    let compatible = unsatisfied_codes.is_empty();

    CompatibilityCheckResult {
        compatible,
        matched_template,
        satisfied_codes,
        unsatisfied_codes,
        conditions,
        decision_record_id,
    }
}

/// Find the first template whose required DUO set matches the dataset profile codes.
pub fn find_matching_template<'a>(
    templates: &'a [AgreementTemplate],
    dataset: &PolicyProfile,
) -> Option<&'a AgreementTemplate> {
    let dataset_codes: HashSet<DuoCode> = dataset.duo_codes.iter().map(|a| a.code).collect();
    templates.iter().find(|tmpl| {
        let required: HashSet<DuoCode> = tmpl.required_duo_codes.iter().copied().collect();
        if !required.iter().all(|code| dataset_codes.contains(code)) {
            return false;
        }
        if tmpl.allowed_duo_codes.is_empty() {
            return true;
        }
        let allowed: HashSet<DuoCode> = tmpl.allowed_duo_codes.iter().copied().collect();
        dataset_codes
            .iter()
            .all(|code| required.contains(code) || allowed.contains(code))
    })
}

/// Evaluate dataset vs project DUO codes for ADS auto-approval.
///
/// Uses the same permission hierarchy as [`check_compatibility`]. The OWL-backed
/// HTTP matcher in `duo-service` remains the catalog-aware API for resource services.
pub fn evaluate_duo_codes(
    dataset_duo: &[DuoCode],
    project_duo: &[DuoCode],
    threshold: u8,
) -> crate::ads::DuoEvaluationResult {
    use crate::ads::DuoEvaluationResult;

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
            if let Some(note) = procedural_modifier(*required) {
                procedural_modifiers.push(note.to_string());
            } else {
                procedural_modifiers.push(format!("missing modifier {required}"));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agreement::DuoCodeAssertion;
    use crate::visa::VisaType;
    use chrono::{TimeZone, Utc};

    fn profile(id: &str, assertions: Vec<DuoCodeAssertion>) -> PolicyProfile {
        PolicyProfile {
            id: id.to_string(),
            owner: "test".to_string(),
            duo_codes: assertions,
            based_on_template: None,
            version: "1".to_string(),
            effective_date: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            source_document_ref: None,
            visa_types: Vec::new(),
        }
    }

    fn assertion(code: DuoCode) -> DuoCodeAssertion {
        DuoCodeAssertion {
            code,
            modifier_value: None,
            rationale: None,
        }
    }

    #[test]
    fn hmb_requester_satisfies_gru_dataset() {
        let requester = profile("req", vec![assertion(DuoCode::Hmb)]);
        let dataset = profile("ds", vec![assertion(DuoCode::Gru)]);
        let result = check_compatibility(&requester, &dataset, None);
        assert!(result.compatible);
        assert!(result.unsatisfied_codes.is_empty());
    }

    #[test]
    fn gru_requester_does_not_satisfy_hmb_dataset() {
        let requester = profile("req", vec![assertion(DuoCode::Gru)]);
        let dataset = profile("ds", vec![assertion(DuoCode::Hmb)]);
        let result = check_compatibility(&requester, &dataset, None);
        assert!(!result.compatible);
        assert!(result.unsatisfied_codes.contains(&DuoCode::Hmb));
    }

    #[test]
    fn missing_modifier_is_incompatible() {
        let requester = profile("req", vec![assertion(DuoCode::Gru)]);
        let dataset = profile("ds", vec![assertion(DuoCode::Gru), assertion(DuoCode::Npu)]);
        let result = check_compatibility(&requester, &dataset, None);
        assert!(!result.compatible);
        assert!(result.unsatisfied_codes.contains(&DuoCode::Npu));
    }

    #[test]
    fn compatible_with_conditions_for_irb_modifier() {
        let requester = profile(
            "req",
            vec![assertion(DuoCode::Gru), assertion(DuoCode::Irb)],
        );
        let dataset = profile("ds", vec![assertion(DuoCode::Gru), assertion(DuoCode::Irb)]);
        let result = check_compatibility(&requester, &dataset, None);
        assert!(result.compatible);
        assert!(!result.conditions.is_empty());
    }

    #[test]
    fn nres_dataset_is_compatible_with_any_requester() {
        let requester = profile("req", vec![assertion(DuoCode::Hmb)]);
        let dataset = profile("ds", vec![assertion(DuoCode::Nres)]);
        let result = check_compatibility(&requester, &dataset, None);
        assert!(result.compatible);
    }

    #[test]
    fn template_match_adds_visa_condition_when_missing() {
        let requester = profile(
            "req",
            vec![assertion(DuoCode::Gru), assertion(DuoCode::Npu)],
        );
        let dataset = profile("ds", vec![assertion(DuoCode::Gru), assertion(DuoCode::Npu)]);
        let template = AgreementTemplate {
            id: "tmpl".to_string(),
            name: "T".to_string(),
            version: "1".to_string(),
            description: "d".to_string(),
            required_duo_codes: vec![DuoCode::Gru, DuoCode::Npu],
            allowed_duo_codes: vec![],
            required_visa_types: vec![VisaType::ControlledAccessGrants],
            reference_url: None,
            is_illustrative: false,
        };
        let result = check_compatibility(&requester, &dataset, Some(&template));
        assert!(result.compatible);
        assert_eq!(result.matched_template.as_deref(), Some("tmpl"));
        assert!(result
            .conditions
            .iter()
            .any(|c| c.contains("ControlledAccessGrants")));
    }

    #[test]
    fn illustrative_template_adds_condition() {
        let requester = profile("req", vec![assertion(DuoCode::Gru)]);
        let dataset = profile("ds", vec![assertion(DuoCode::Gru)]);
        let template = AgreementTemplate {
            id: "illustrative".to_string(),
            name: "I".to_string(),
            version: "1".to_string(),
            description: "d".to_string(),
            required_duo_codes: vec![DuoCode::Gru],
            allowed_duo_codes: vec![],
            required_visa_types: vec![],
            reference_url: None,
            is_illustrative: true,
        };
        let result = check_compatibility(&requester, &dataset, Some(&template));
        assert!(result.conditions.iter().any(|c| c.contains("illustrative")));
    }

    #[test]
    fn find_matching_template_rejects_extra_codes_not_in_allowed() {
        let dataset = profile("ds", vec![assertion(DuoCode::Gru), assertion(DuoCode::Ncu)]);
        let templates = [AgreementTemplate {
            id: "tmpl".to_string(),
            name: "T".to_string(),
            version: "1".to_string(),
            description: "d".to_string(),
            required_duo_codes: vec![DuoCode::Gru],
            allowed_duo_codes: vec![DuoCode::Npu],
            required_visa_types: vec![],
            reference_url: None,
            is_illustrative: false,
        }];
        assert!(find_matching_template(&templates, &dataset).is_none());

        let allowed = profile(
            "ds2",
            vec![assertion(DuoCode::Gru), assertion(DuoCode::Npu)],
        );
        assert_eq!(
            find_matching_template(&templates, &allowed).map(|t| t.id.as_str()),
            Some("tmpl")
        );
    }

    #[test]
    fn evaluate_duo_codes_hmb_satisfies_gru() {
        let result = evaluate_duo_codes(&[DuoCode::Gru], &[DuoCode::Hmb], 100);
        assert!(result.compatible);
        assert!(result.auto_approvable);
    }

    #[test]
    fn evaluate_duo_codes_gru_does_not_satisfy_hmb() {
        let result = evaluate_duo_codes(&[DuoCode::Hmb], &[DuoCode::Gru], 100);
        assert!(!result.compatible);
    }
}
