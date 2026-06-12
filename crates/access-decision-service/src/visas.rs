// SPDX-License-Identifier: Apache-2.0

//! Grant-to-visa conversion for GA4GH AAI passport assembly.

use chrono::Utc;
use ga4gh_types::{Grant, Researcher, ResearcherVisasResponse, VisaClaim, VisaType};

use crate::config::VisaExportConfig;

/// Build unsigned visa claims from active grants and affiliations.
pub fn researcher_visas(
    researcher: &Researcher,
    grants: &[Grant],
    config: &VisaExportConfig,
) -> ResearcherVisasResponse {
    let now = Utc::now().timestamp();
    let mut visas = Vec::new();

    for affiliation in &researcher.affiliations {
        visas.push(VisaClaim {
            r#type: VisaType::AffiliationAndRole,
            value: format!("{}@{}", affiliation.role, affiliation.organization),
            source: config.default_source_url.clone(),
            by: None,
            conditions: None,
            asserted: now,
        });
    }

    for grant in grants {
        if grant.revoked_at.is_some() {
            continue;
        }
        if let Some(expires) = grant.expires_at {
            if expires <= Utc::now() {
                continue;
            }
        }

        let value = grant
            .resource_scope
            .clone()
            .unwrap_or_else(|| grant.dataset_id.to_string());

        visas.push(VisaClaim {
            r#type: VisaType::ControlledAccessGrants,
            value,
            source: config.default_source_url.clone(),
            by: None,
            conditions: None,
            asserted: grant.created_at.timestamp(),
        });
    }

    ResearcherVisasResponse {
        researcher_id: researcher.id.clone(),
        visas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ga4gh_types::{GrantSource, ResearcherAffiliation};
    use uuid::Uuid;

    #[test]
    fn exports_controlled_access_and_affiliation_visas() {
        let researcher = Researcher {
            id: "researcher@example.org".to_string(),
            display_name: None,
            email: None,
            affiliations: vec![ResearcherAffiliation {
                organization: "uni-heidelberg.de".to_string(),
                role: "faculty".to_string(),
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let grant = Grant {
            id: Uuid::new_v4(),
            researcher_id: researcher.id.clone(),
            dataset_id: Uuid::new_v4(),
            request_id: None,
            source: GrantSource::DacApproval,
            duo_codes: vec![],
            resource_scope: Some("drs:dataset:abc".to_string()),
            expires_at: None,
            revoked_at: None,
            created_at: Utc::now(),
        };
        let response = researcher_visas(
            &researcher,
            &[grant],
            &VisaExportConfig {
                default_source_url: "https://ads.test".to_string(),
            },
        );
        assert_eq!(response.visas.len(), 2);
        assert_eq!(response.visas[0].r#type, VisaType::AffiliationAndRole);
        assert_eq!(response.visas[1].r#type, VisaType::ControlledAccessGrants);
    }
}
