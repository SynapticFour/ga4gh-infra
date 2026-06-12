// SPDX-License-Identifier: Apache-2.0

//! Passport policy evaluation helpers.

use ga4gh_types::{DuoCode, Visa, VisaType};

/// Policy expression evaluated against a set of validated Visas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyCheck {
    /// Require a controlled-access grant for a dataset identifier.
    HasControlledAccess {
        /// Dataset or resource identifier encoded in the visa `value`.
        dataset_id: String,
    },
    /// Require an affiliation visa whose value matches the domain suffix.
    HasAffiliation {
        /// Email-domain suffix such as `uni-heidelberg.de`.
        domain: String,
    },
    /// Require a visa whose value references a DUO permission code.
    HasDuoPermission {
        /// Required DUO shorthand code.
        code: DuoCode,
    },
    /// All nested checks must match.
    All(Vec<PolicyCheck>),
    /// Any nested check may match.
    Any(Vec<PolicyCheck>),
}

/// Result of evaluating a [`PolicyCheck`] against a visa set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyResult {
    /// Whether access is permitted.
    pub permitted: bool,
    /// Visas that satisfied the policy.
    pub matched_visas: Vec<Visa>,
    /// Human-readable explanation of the decision.
    pub reason: String,
}

/// Evaluate a policy against the provided visas.
pub fn evaluate_policy(visas: &[Visa], policy: &PolicyCheck) -> PolicyResult {
    match policy {
        PolicyCheck::HasControlledAccess { dataset_id } => {
            let matched: Vec<_> = visas
                .iter()
                .filter(|visa| {
                    visa.claim.r#type == VisaType::ControlledAccessGrants
                        && visa.claim.value == *dataset_id
                })
                .cloned()
                .collect();
            let permitted = !matched.is_empty();
            PolicyResult {
                permitted,
                matched_visas: matched,
                reason: if permitted {
                    format!("controlled access grant found for dataset `{dataset_id}`")
                } else {
                    format!("no ControlledAccessGrants visa for dataset `{dataset_id}`")
                },
            }
        }
        PolicyCheck::HasAffiliation { domain } => {
            let matched: Vec<_> = visas
                .iter()
                .filter(|visa| {
                    visa.claim.r#type == VisaType::AffiliationAndRole
                        && affiliation_matches_domain(&visa.claim.value, domain)
                })
                .cloned()
                .collect();
            let permitted = !matched.is_empty();
            PolicyResult {
                permitted,
                matched_visas: matched,
                reason: if permitted {
                    format!("affiliation visa matched domain `{domain}`")
                } else {
                    format!("no AffiliationAndRole visa for domain `{domain}`")
                },
            }
        }
        PolicyCheck::HasDuoPermission { code } => {
            let code_str = code.as_str();
            let matched: Vec<_> = visas
                .iter()
                .filter(|visa| {
                    visa.claim.value == code_str
                        || visa.claim.value.contains(code_str)
                        || visa.claim.value.contains(&format!("DUO:{code_str}"))
                })
                .cloned()
                .collect();
            let permitted = !matched.is_empty();
            PolicyResult {
                permitted,
                matched_visas: matched,
                reason: if permitted {
                    format!("visa references DUO permission `{code_str}`")
                } else {
                    format!("no visa references DUO permission `{code_str}`")
                },
            }
        }
        PolicyCheck::All(checks) => {
            let mut matched = Vec::new();
            for check in checks {
                let result = evaluate_policy(visas, check);
                if !result.permitted {
                    return PolicyResult {
                        permitted: false,
                        matched_visas: matched,
                        reason: result.reason,
                    };
                }
                matched.extend(result.matched_visas);
            }
            PolicyResult {
                permitted: true,
                matched_visas: matched,
                reason: "all policy checks matched".to_string(),
            }
        }
        PolicyCheck::Any(checks) => {
            let mut reasons = Vec::new();
            let mut matched = Vec::new();
            for check in checks {
                let result = evaluate_policy(visas, check);
                if result.permitted {
                    matched.extend(result.matched_visas);
                    return PolicyResult {
                        permitted: true,
                        matched_visas: matched,
                        reason: result.reason,
                    };
                }
                reasons.push(result.reason);
            }
            PolicyResult {
                permitted: false,
                matched_visas: matched,
                reason: reasons.join("; "),
            }
        }
    }
}

fn affiliation_matches_domain(value: &str, domain: &str) -> bool {
    value
        .split('@')
        .nth(1)
        .is_some_and(|suffix| suffix == domain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ga4gh_types::VisaClaim;

    fn visa(visa_type: VisaType, value: &str) -> Visa {
        Visa {
            sub: "researcher@example.org".to_string(),
            iss: "https://visas.example.org".to_string(),
            iat: 1_700_000_000,
            exp: 1_800_000_000,
            jti: "visa-jti".to_string(),
            claim: VisaClaim {
                r#type: visa_type,
                asserted: 1_699_999_000,
                value: value.to_string(),
                source: "https://visas.example.org".to_string(),
                by: None,
                conditions: None,
            },
            scope: None,
            jku: None,
        }
    }

    #[test]
    fn controlled_access_policy_matches_dataset() {
        let visas = vec![visa(VisaType::ControlledAccessGrants, "dataset-abc")];
        let result = evaluate_policy(
            &visas,
            &PolicyCheck::HasControlledAccess {
                dataset_id: "dataset-abc".to_string(),
            },
        );
        assert!(result.permitted);
    }

    #[test]
    fn affiliation_policy_matches_domain_suffix() {
        let visas = vec![visa(
            VisaType::AffiliationAndRole,
            "faculty@uni-heidelberg.de",
        )];
        let result = evaluate_policy(
            &visas,
            &PolicyCheck::HasAffiliation {
                domain: "uni-heidelberg.de".to_string(),
            },
        );
        assert!(result.permitted);
    }

    #[test]
    fn duo_permission_policy_matches_code() {
        let visas = vec![visa(VisaType::ControlledAccessGrants, "DUO:HMB")];
        let result = evaluate_policy(
            &visas,
            &PolicyCheck::HasDuoPermission {
                code: ga4gh_types::DuoCode::Hmb,
            },
        );
        assert!(result.permitted);
    }

    #[test]
    fn all_policy_requires_every_check() {
        let visas = vec![visa(VisaType::ControlledAccessGrants, "dataset-abc")];
        let result = evaluate_policy(
            &visas,
            &PolicyCheck::All(vec![
                PolicyCheck::HasControlledAccess {
                    dataset_id: "dataset-abc".to_string(),
                },
                PolicyCheck::HasAffiliation {
                    domain: "uni-heidelberg.de".to_string(),
                },
            ]),
        );
        assert!(!result.permitted);
    }

    #[test]
    fn any_policy_matches_first_success() {
        let visas = vec![visa(VisaType::ControlledAccessGrants, "dataset-abc")];
        let result = evaluate_policy(
            &visas,
            &PolicyCheck::Any(vec![
                PolicyCheck::HasAffiliation {
                    domain: "example.org".to_string(),
                },
                PolicyCheck::HasControlledAccess {
                    dataset_id: "dataset-abc".to_string(),
                },
            ]),
        );
        assert!(result.permitted);
    }

    #[test]
    fn empty_visa_list_denies_controlled_access() {
        let result = evaluate_policy(
            &[],
            &PolicyCheck::HasControlledAccess {
                dataset_id: "dataset-abc".to_string(),
            },
        );
        assert!(!result.permitted);
    }
}
