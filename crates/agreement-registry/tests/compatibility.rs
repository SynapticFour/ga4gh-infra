//! Integration-style test using seed template and in-memory registry (no HTTP).

use chrono::Utc;
use ga4gh_types::{CompatibilityCheckRequest, DuoCode, DuoCodeAssertion, PolicyProfile, VisaType};
use agreement_registry::InMemoryRegistry;

fn assertion(code: DuoCode, modifier_value: Option<&str>) -> DuoCodeAssertion {
    DuoCodeAssertion {
        code,
        modifier_value: modifier_value.map(str::to_string),
        rationale: Some("seed integration test".to_string()),
    }
}

fn profile(
    id: &str,
    template_id: &str,
    codes: Vec<DuoCodeAssertion>,
    visa_types: Vec<VisaType>,
) -> PolicyProfile {
    PolicyProfile {
        id: id.to_string(),
        owner: id.to_string(),
        duo_codes: codes,
        based_on_template: Some(template_id.to_string()),
        version: "1.0.0".to_string(),
        effective_date: Utc::now(),
        source_document_ref: None,
        visa_types,
    }
}

#[tokio::test]
async fn compatibility_check_against_duos_gru_ncu_seed_template() {
    let mut registry = InMemoryRegistry::new()
        .with_seed_templates()
        .expect("seed templates");

    let template_id = "duos-dbgap-gru-ncu-v1";

    registry.register_profile(profile(
        "researcher.duos-pilot",
        template_id,
        vec![assertion(DuoCode::Gru, None), assertion(DuoCode::Ncu, None)],
        vec![VisaType::ControlledAccessGrants, VisaType::ResearcherStatus],
    ));

    registry.register_profile(profile(
        "dataset.phs000424-style",
        template_id,
        vec![assertion(DuoCode::Gru, None), assertion(DuoCode::Ncu, None)],
        vec![],
    ));

    let result = registry
        .compatibility_check(
            &CompatibilityCheckRequest {
                requester_profile_id: "researcher.duos-pilot".to_string(),
                dataset_profile_id: "dataset.phs000424-style".to_string(),
            },
            Utc::now(),
            Some("integration-test".to_string()),
        )
        .expect("check");

    assert!(result.compatible);
    assert_eq!(result.matched_template.as_deref(), Some(template_id));
    assert!(result.unsatisfied_codes.is_empty());
    assert!(result.satisfied_codes.contains(&DuoCode::Gru));
    assert!(result.satisfied_codes.contains(&DuoCode::Ncu));

    let decisions = registry.list_decisions(Some("researcher.duos-pilot"));
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].id, result.decision_record_id);
}

#[tokio::test]
async fn incompatible_when_commercial_use_missing_ncu() {
    let mut registry = InMemoryRegistry::new()
        .with_seed_templates()
        .expect("seed templates");

    let template_id = "duos-dbgap-gru-ncu-v1";

    registry.register_profile(profile(
        "researcher.commercial-missing",
        template_id,
        vec![assertion(DuoCode::Gru, None)],
        vec![VisaType::ControlledAccessGrants],
    ));

    registry.register_profile(profile(
        "dataset.ncu-required",
        template_id,
        vec![assertion(DuoCode::Gru, None), assertion(DuoCode::Ncu, None)],
        vec![],
    ));

    let result = registry
        .compatibility_check(
            &CompatibilityCheckRequest {
                requester_profile_id: "researcher.commercial-missing".to_string(),
                dataset_profile_id: "dataset.ncu-required".to_string(),
            },
            Utc::now(),
            None,
        )
        .expect("check");

    assert!(!result.compatible);
    assert!(result.unsatisfied_codes.contains(&DuoCode::Ncu));
}
