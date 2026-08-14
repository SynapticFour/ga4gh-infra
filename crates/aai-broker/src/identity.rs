// SPDX-License-Identifier: Apache-2.0

//! Upstream claim mapping into broker identity representation.

use std::collections::HashMap;

use serde_json::Value;

use crate::config::UpstreamIdpConfig;

/// Broker-internal identity derived from an upstream IdP token or userinfo response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearcherIdentity {
    /// Subject identifier used for visa lookup and Passport `sub`.
    pub sub: String,
    /// Email address when available from upstream claims.
    pub email: Option<String>,
    /// Display name when available from upstream claims.
    pub display_name: Option<String>,
    /// Institutional affiliation when available from upstream claims.
    pub affiliation: Option<String>,
    /// Group / entitlement values mapped from upstream (e.g. `groups`).
    pub groups: Vec<String>,
}

impl ResearcherIdentity {
    /// Map upstream JWT / userinfo claims using the configured claim mapping table.
    pub fn from_claims(
        claims: &HashMap<String, Value>,
        mapping: &HashMap<String, String>,
    ) -> Option<Self> {
        let sub = mapped_string(claims, mapping, "sub")?;
        let email = mapped_string(claims, mapping, "email");
        let display_name = mapped_string(claims, mapping, "name");
        let affiliation = mapped_string(claims, mapping, "affiliation");
        let groups = mapped_groups(claims, mapping);

        Some(Self {
            sub,
            email,
            display_name,
            affiliation,
            groups,
        })
    }

    /// Apply default claim mapping for an upstream IdP configuration.
    pub fn from_upstream(idp: &UpstreamIdpConfig, claims: &HashMap<String, Value>) -> Option<Self> {
        let mut mapping = idp.claim_mapping.clone();
        mapping
            .entry("sub".to_string())
            .or_insert_with(|| "sub".to_string());
        mapping
            .entry("groups".to_string())
            .or_insert_with(|| "groups".to_string());
        mapping
            .entry("name".to_string())
            .or_insert_with(|| "name".to_string());
        Self::from_claims(claims, &mapping)
    }
}

fn mapped_string(
    claims: &HashMap<String, Value>,
    mapping: &HashMap<String, String>,
    field: &str,
) -> Option<String> {
    let claim_name = mapping.get(field).map(String::as_str).unwrap_or(field);
    claims
        .get(claim_name)
        .and_then(value_to_string)
        .filter(|value| !value.is_empty())
}

fn mapped_groups(
    claims: &HashMap<String, Value>,
    mapping: &HashMap<String, String>,
) -> Vec<String> {
    let claim_name = mapping
        .get("groups")
        .map(String::as_str)
        .unwrap_or("groups");
    match claims.get(claim_name) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(value_to_string)
            .filter(|value| !value.is_empty())
            .collect(),
        Some(Value::String(text)) if !text.is_empty() => vec![text.clone()],
        _ => Vec::new(),
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Array(items) => items.first().and_then(value_to_string),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claims_map_from_json(value: &Value) -> HashMap<String, Value> {
        match value {
            Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => HashMap::new(),
        }
    }

    #[test]
    fn maps_configured_claims() {
        let claims = claims_map_from_json(&json!({
            "sub": "upstream-subject",
            "email": "researcher@example.org",
            "name": "Ada Researcher",
            "groups": ["ega-dac", "ga4gh-infra-admins"],
            "eduperson_scoped_affiliation": ["faculty@uni-heidelberg.de"]
        }));

        let mapping = HashMap::from([
            ("sub".to_string(), "sub".to_string()),
            ("email".to_string(), "email".to_string()),
            (
                "affiliation".to_string(),
                "eduperson_scoped_affiliation".to_string(),
            ),
        ]);

        let identity = ResearcherIdentity::from_claims(&claims, &mapping).expect("identity");
        assert_eq!(identity.sub, "upstream-subject");
        assert_eq!(identity.email.as_deref(), Some("researcher@example.org"));
        assert_eq!(identity.display_name.as_deref(), Some("Ada Researcher"));
        assert_eq!(
            identity.affiliation.as_deref(),
            Some("faculty@uni-heidelberg.de")
        );
        assert_eq!(
            identity.groups,
            vec!["ega-dac".to_string(), "ga4gh-infra-admins".to_string()]
        );
    }
}
