// SPDX-License-Identifier: Apache-2.0

//! In-memory policy profile and template store (pre-HTTP service implementation).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ga4gh_types::{
    check_compatibility, AgreementTemplate, CompatibilityCheckRequest, CompatibilityCheckResult,
    DecisionRecord, PolicyProfile,
};

use crate::error::AgreementRegistryError;

/// Simple in-memory registry for tests and future service backing store.
#[derive(Debug, Default)]
pub struct InMemoryRegistry {
    profiles: HashMap<String, PolicyProfile>,
    templates: HashMap<String, AgreementTemplate>,
    decisions: HashMap<String, DecisionRecord>,
}

impl InMemoryRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load curated seed templates into the registry.
    pub fn with_seed_templates(mut self) -> Result<Self, AgreementRegistryError> {
        for template in crate::seeds::seed_templates()? {
            self.templates.insert(template.id.clone(), template);
        }
        Ok(self)
    }

    /// Register a policy profile.
    pub fn register_profile(&mut self, profile: PolicyProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    /// Register an agreement template.
    pub fn register_template(&mut self, template: AgreementTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    /// List all policy profiles.
    pub fn list_profiles(&self) -> Vec<&PolicyProfile> {
        let mut out: Vec<_> = self.profiles.values().collect();
        out.sort_by_key(|p| p.id.as_str());
        out
    }

    /// Look up a policy profile by id.
    pub fn get_profile(&self, id: &str) -> Result<&PolicyProfile, AgreementRegistryError> {
        self.profiles
            .get(id)
            .ok_or_else(|| AgreementRegistryError::NotFound(id.to_string()))
    }

    /// List all templates.
    pub fn list_templates(&self) -> Vec<&AgreementTemplate> {
        let mut out: Vec<_> = self.templates.values().collect();
        out.sort_by_key(|t| t.id.as_str());
        out
    }

    /// Look up a template by id.
    pub fn get_template(&self, id: &str) -> Result<&AgreementTemplate, AgreementRegistryError> {
        self.templates
            .get(id)
            .ok_or_else(|| AgreementRegistryError::NotFound(id.to_string()))
    }

    /// Run compatibility check and persist a decision record.
    pub fn compatibility_check(
        &mut self,
        request: &CompatibilityCheckRequest,
        checked_at: DateTime<Utc>,
        checked_by: Option<String>,
    ) -> Result<CompatibilityCheckResult, AgreementRegistryError> {
        let requester = self.get_profile(&request.requester_profile_id)?.clone();
        let dataset = self.get_profile(&request.dataset_profile_id)?.clone();

        let template = dataset
            .based_on_template
            .as_deref()
            .and_then(|id| self.templates.get(id));

        let result = check_compatibility(&requester, &dataset, template);

        let record = DecisionRecord {
            id: result.decision_record_id.clone(),
            requester_profile_id: request.requester_profile_id.clone(),
            dataset_profile_id: request.dataset_profile_id.clone(),
            result: result.clone(),
            checked_at,
            checked_by,
        };
        self.decisions.insert(record.id.clone(), record);
        Ok(result)
    }

    /// List decision records optionally filtered by profile id (requester or dataset side).
    pub fn list_decisions(&self, profile_id: Option<&str>) -> Vec<&DecisionRecord> {
        self.decisions
            .values()
            .filter(|record| {
                profile_id.is_none_or(|id| {
                    record.requester_profile_id == id || record.dataset_profile_id == id
                })
            })
            .collect()
    }
}
