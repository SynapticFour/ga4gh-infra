// SPDX-License-Identifier: Apache-2.0

//! Institutional OIDC claim → grant mapping.

use std::collections::BTreeMap;

use chrono::{Duration, Utc};
use ga4gh_types::{Grant, GrantSource, ResearcherSyncRequest};
use serde_json::Value;
use uuid::Uuid;

use crate::error::AdsError;
use crate::store::AdsStore;

/// Upsert researcher profile and apply institutional permission mappings.
pub async fn sync_researcher(
    store: &AdsStore,
    request: &ResearcherSyncRequest,
) -> Result<Vec<Grant>, AdsError> {
    let now = Utc::now();
    let researcher = ga4gh_types::Researcher {
        id: request.sub.clone(),
        display_name: request.display_name.clone(),
        email: request.email.clone(),
        affiliations: request.affiliations.clone(),
        created_at: now,
        updated_at: now,
    };
    store.upsert_researcher(&researcher).await?;
    store
        .apply_institutional_mappings(&request.sub, &request.claims)
        .await
}

/// Extract string claim values from a flat or dotted claim path.
pub fn claim_values(claims: &BTreeMap<String, Value>, path: &str) -> Vec<String> {
    if let Some(value) = claims.get(path) {
        return values_from_json(value);
    }

    if path.contains('.') {
        let mut current =
            Value::Object(claims.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        for segment in path.split('.') {
            current = match current {
                Value::Object(map) => map.get(segment).cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            };
        }
        return values_from_json(&current);
    }

    vec![]
}

fn values_from_json(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Value::Bool(b) => vec![b.to_string()],
        Value::Number(n) => vec![n.to_string()],
        _ => vec![],
    }
}

pub fn grant_from_mapping(
    researcher_id: &str,
    dataset_id: Uuid,
    duo_codes: Vec<ga4gh_types::DuoCode>,
    resource_scope: Option<String>,
    lifetime_seconds: Option<u64>,
) -> Grant {
    Grant {
        id: Uuid::new_v4(),
        researcher_id: researcher_id.to_string(),
        dataset_id,
        request_id: None,
        source: GrantSource::InstitutionalMapping,
        duo_codes,
        resource_scope,
        expires_at: lifetime_seconds.map(|secs| Utc::now() + Duration::seconds(secs as i64)),
        revoked_at: None,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_array_claim_values() {
        let mut claims = BTreeMap::new();
        claims.insert("groups".to_string(), json!(["ega-approved", "staff"]));
        let values = claim_values(&claims, "groups");
        assert_eq!(values, vec!["ega-approved", "staff"]);
    }

    #[test]
    fn reads_dotted_claim_path() {
        let mut claims = BTreeMap::new();
        claims.insert(
            "realm_access".to_string(),
            json!({ "roles": ["researcher", "admin"] }),
        );
        let values = claim_values(&claims, "realm_access.roles");
        assert_eq!(values, vec!["researcher", "admin"]);
    }
}
