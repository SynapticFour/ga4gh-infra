// SPDX-License-Identifier: Apache-2.0

//! Demo policy profiles for development and admin-ui compatibility checks.

use chrono::Utc;
use ga4gh_types::{DuoCode, DuoCodeAssertion, PolicyProfile};

use crate::registry::InMemoryRegistry;

/// Register illustrative requester/dataset GRU profiles used by the admin UI.
pub fn register_demo_profiles(registry: &mut InMemoryRegistry) {
    let now = Utc::now();
    registry.register_profile(PolicyProfile {
        id: "demo.requester.gru".to_string(),
        owner: "demo-researcher".to_string(),
        duo_codes: vec![assertion(DuoCode::Gru)],
        based_on_template: Some("ega-general-research-use-v1".to_string()),
        version: "1.0.0".to_string(),
        effective_date: now,
        source_document_ref: None,
        visa_types: vec![],
    });
    registry.register_profile(PolicyProfile {
        id: "demo.dataset.gru".to_string(),
        owner: "demo-dataset".to_string(),
        duo_codes: vec![assertion(DuoCode::Gru)],
        based_on_template: Some("ega-general-research-use-v1".to_string()),
        version: "1.0.0".to_string(),
        effective_date: now,
        source_document_ref: None,
        visa_types: vec![],
    });
}

fn assertion(code: DuoCode) -> DuoCodeAssertion {
    DuoCodeAssertion {
        code,
        modifier_value: None,
        rationale: None,
    }
}
