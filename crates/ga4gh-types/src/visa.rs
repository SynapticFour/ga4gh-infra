// SPDX-License-Identifier: Apache-2.0

//! GA4GH Visa types.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// Standard GA4GH visa type identifiers defined by the Passport specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisaType {
    /// Affiliation and role within an organization (e.g. `faculty@uni-heidelberg.de`).
    AffiliationAndRole,
    /// Acceptance of terms, policies, or codes of conduct.
    AcceptedTermsAndPolicies,
    /// Researcher status assertion (e.g. Registered Access bona fide researcher).
    ResearcherStatus,
    /// Controlled-access grant for a specific dataset or resource.
    ControlledAccessGrants,
    /// Linked identity assertions across visa issuers.
    LinkedIdentities,
    /// A non-standard or custom visa type name.
    Custom(String),
}

impl VisaType {
    /// Returns the canonical string representation used in visa JWTs.
    pub fn as_str(&self) -> &str {
        match self {
            Self::AffiliationAndRole => "AffiliationAndRole",
            Self::AcceptedTermsAndPolicies => "AcceptedTermsAndPolicies",
            Self::ResearcherStatus => "ResearcherStatus",
            Self::ControlledAccessGrants => "ControlledAccessGrants",
            Self::LinkedIdentities => "LinkedIdentities",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for VisaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when parsing an invalid visa type string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid visa type: {0}")]
pub struct VisaTypeError(pub String);

impl FromStr for VisaType {
    type Err = VisaTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "AffiliationAndRole" => Self::AffiliationAndRole,
            "AcceptedTermsAndPolicies" => Self::AcceptedTermsAndPolicies,
            "ResearcherStatus" => Self::ResearcherStatus,
            "ControlledAccessGrants" => Self::ControlledAccessGrants,
            "LinkedIdentities" => Self::LinkedIdentities,
            other if !other.is_empty() => Self::Custom(other.to_string()),
            _ => return Err(VisaTypeError(s.to_string())),
        })
    }
}

impl Serialize for VisaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for VisaType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisaTypeVisitor;

        impl Visitor<'_> for VisaTypeVisitor {
            type Value = VisaType;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a GA4GH visa type string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                VisaType::from_str(value).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(VisaTypeVisitor)
    }
}

/// Authority level for a visa assertion within the `source` organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VisaAuthority {
    /// The visa identity made the assertion themselves.
    #[serde(rename = "self")]
    Self_,
    /// A peer at the source organization made the assertion.
    Peer,
    /// The source organization's information system made the assertion.
    System,
    /// A signing official with direct organizational authority.
    So,
    /// A Data Access Committee or grantee decision-maker.
    Dac,
}

/// Match type prefix used in visa `conditions` claim values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionMatchType {
    /// Case-sensitive full string match.
    Const,
    /// Full-string pattern match with `?` and `*` wildcards.
    Pattern,
    /// Pattern match against semicolon-separated substrings.
    SplitPattern,
}

/// A single claim match expression within a visa condition clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionMatch {
    /// Matching algorithm for the condition value.
    pub r#type: ConditionMatchType,
    /// Suffix value after the match-type prefix.
    pub value: String,
}

impl ConditionMatch {
    /// Parse a condition claim value of the form `<match-type>:<match-value>`.
    pub fn parse(raw: &str) -> Option<Self> {
        let (prefix, suffix) = raw.split_once(':')?;
        let match_type = match prefix {
            "const" => ConditionMatchType::Const,
            "pattern" => ConditionMatchType::Pattern,
            "split_pattern" => ConditionMatchType::SplitPattern,
            _ => return None,
        };
        Some(Self {
            r#type: match_type,
            value: suffix.to_string(),
        })
    }

    /// Format this match as a condition claim value string.
    pub fn to_condition_string(&self) -> String {
        let prefix = match self.r#type {
            ConditionMatchType::Const => "const",
            ConditionMatchType::Pattern => "pattern",
            ConditionMatchType::SplitPattern => "split_pattern",
        };
        format!("{prefix}:{}", self.value)
    }
}

/// One AND-clause within visa conditions (disjunctive normal form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaConditionClause {
    /// Visa type that must be matched in the passport.
    pub r#type: VisaType,
    /// Additional visa-object claim matches keyed by claim name.
    #[serde(flatten, default)]
    pub matches: BTreeMap<String, String>,
}

/// Visa conditions in disjunctive normal form: OR of AND-clauses.
pub type VisaConditions = Vec<Vec<VisaConditionClause>>;

/// The `ga4gh_visa_v1` object inside a visa JWT payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaClaim {
    /// Semantic type of the visa assertion.
    pub r#type: VisaType,
    /// Seconds since Unix epoch when the assertion source made the claim.
    pub asserted: i64,
    /// Assertion value; format depends on the visa type.
    pub value: String,
    /// URL identifying the organization that made the assertion.
    pub source: String,
    /// Optional authority level within the source organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<VisaAuthority>,
    /// Optional conditions restricting when this visa is valid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<VisaConditions>,
}

/// JWT payload claims for a decoded GA4GH Visa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaJwtClaims {
    /// Subject identifier for the researcher within the issuer's system.
    pub sub: String,
    /// Issuer URL of the visa issuer.
    pub iss: String,
    /// Issued-at timestamp (seconds since Unix epoch).
    pub iat: i64,
    /// Expiration timestamp (seconds since Unix epoch).
    pub exp: i64,
    /// Unique token identifier.
    pub jti: String,
    /// GA4GH visa object claim.
    #[serde(rename = "ga4gh_visa_v1")]
    pub ga4gh_visa_v1: VisaClaim,
    /// OAuth scope claim; required when `jku` is absent per AAI profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// JSON Web Key Set URL; required when `scope` is absent per AAI profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jku: Option<String>,
}

/// A decoded GA4GH Visa with standard JWT claims and the `ga4gh_visa_v1` object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "VisaJwtClaims", into = "VisaJwtClaims")]
pub struct Visa {
    /// Subject identifier for the researcher within the issuer's system.
    pub sub: String,
    /// Issuer URL of the visa issuer.
    pub iss: String,
    /// Issued-at timestamp (seconds since Unix epoch).
    pub iat: i64,
    /// Expiration timestamp (seconds since Unix epoch).
    pub exp: i64,
    /// Unique token identifier.
    pub jti: String,
    /// Parsed `ga4gh_visa_v1` claim.
    pub claim: VisaClaim,
    /// OAuth scope claim, when present.
    pub scope: Option<String>,
    /// JSON Web Key Set URL, when present.
    pub jku: Option<String>,
}

impl Visa {
    /// Construct a [`Visa`] from decoded JWT claims.
    pub fn from_claims(claims: VisaJwtClaims) -> Self {
        Self {
            sub: claims.sub,
            iss: claims.iss,
            iat: claims.iat,
            exp: claims.exp,
            jti: claims.jti,
            claim: claims.ga4gh_visa_v1,
            scope: claims.scope,
            jku: claims.jku,
        }
    }

    /// Convert this visa back into JWT claim form for serialization.
    pub fn into_claims(self) -> VisaJwtClaims {
        VisaJwtClaims {
            sub: self.sub,
            iss: self.iss,
            iat: self.iat,
            exp: self.exp,
            jti: self.jti,
            ga4gh_visa_v1: self.claim,
            scope: self.scope,
            jku: self.jku,
        }
    }
}

impl From<VisaJwtClaims> for Visa {
    fn from(claims: VisaJwtClaims) -> Self {
        Self::from_claims(claims)
    }
}

impl From<Visa> for VisaJwtClaims {
    fn from(visa: Visa) -> Self {
        visa.into_claims()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visa_round_trip() {
        let visa = Visa {
            sub: "researcher@example.org".to_string(),
            iss: "https://visas.example.org".to_string(),
            iat: 1_700_000_000,
            exp: 1_700_003_600,
            jti: "visa-jti-001".to_string(),
            claim: VisaClaim {
                r#type: VisaType::AffiliationAndRole,
                asserted: 1_699_999_000,
                value: "faculty@uni-heidelberg.de".to_string(),
                source: "https://visas.example.org".to_string(),
                by: Some(VisaAuthority::So),
                conditions: None,
            },
            scope: None,
            jku: Some("https://visas.example.org/jwks.json".to_string()),
        };

        let json = serde_json::to_string(&visa).expect("serialize");
        let decoded: Visa = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(visa, decoded);
    }

    #[test]
    fn visa_type_standard_round_trip() {
        for ty in [
            VisaType::AffiliationAndRole,
            VisaType::AcceptedTermsAndPolicies,
            VisaType::ResearcherStatus,
            VisaType::ControlledAccessGrants,
            VisaType::LinkedIdentities,
        ] {
            let json = serde_json::to_string(&ty).expect("serialize");
            let decoded: VisaType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(ty, decoded);
        }
    }

    #[test]
    fn visa_type_custom_round_trip() {
        let custom = VisaType::Custom("MyOrgCustomVisa".to_string());
        let json = serde_json::to_string(&custom).expect("serialize");
        let decoded: VisaType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(custom, decoded);
    }

    #[test]
    fn visa_claim_round_trip() {
        let claim = VisaClaim {
            r#type: VisaType::ControlledAccessGrants,
            asserted: 1_700_000_000,
            value: "dataset-123".to_string(),
            source: "https://dac.example.org".to_string(),
            by: Some(VisaAuthority::Dac),
            conditions: Some(vec![vec![VisaConditionClause {
                r#type: VisaType::AffiliationAndRole,
                matches: BTreeMap::from([(
                    "value".to_string(),
                    "const:faculty@uni-heidelberg.de".to_string(),
                )]),
            }]]),
        };

        let json = serde_json::to_string(&claim).expect("serialize");
        let decoded: VisaClaim = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(claim, decoded);
    }

    #[test]
    fn visa_jwt_claims_round_trip() {
        let claims = VisaJwtClaims {
            sub: "researcher@example.org".to_string(),
            iss: "https://visas.example.org".to_string(),
            iat: 1_700_000_000,
            exp: 1_700_003_600,
            jti: "visa-jti-001".to_string(),
            ga4gh_visa_v1: VisaClaim {
                r#type: VisaType::ResearcherStatus,
                asserted: 1_699_999_000,
                value: "https://doi.org/10.1038/s41431-018-0219-y".to_string(),
                source: "https://visas.example.org".to_string(),
                by: Some(VisaAuthority::So),
                conditions: None,
            },
            scope: Some("openid".to_string()),
            jku: None,
        };

        let json = serde_json::to_string(&claims).expect("serialize");
        let decoded: VisaJwtClaims = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(claims, decoded);

        let visa = Visa::from_claims(decoded);
        let round_trip = VisaJwtClaims::from(visa);
        assert_eq!(claims, round_trip);
    }

    #[test]
    fn condition_match_parse_and_format() {
        let raw = "pattern:faculty@*";
        let parsed = ConditionMatch::parse(raw).expect("parse");
        assert_eq!(parsed.r#type, ConditionMatchType::Pattern);
        assert_eq!(parsed.value, "faculty@*");
        assert_eq!(parsed.to_condition_string(), raw);
    }

    #[test]
    fn visa_type_deserializes_custom_values() {
        let decoded: VisaType =
            serde_json::from_str(r#""CustomInstituteVisa""#).expect("deserialize");
        assert_eq!(decoded, VisaType::Custom("CustomInstituteVisa".to_string()));
    }

    #[test]
    fn visa_type_rejects_empty_string() {
        assert!(serde_json::from_str::<VisaType>(r#""""#).is_err());
    }

    #[test]
    fn visa_jwt_claims_omit_optional_scope_and_jku() {
        let claims = VisaJwtClaims {
            sub: "researcher@example.org".to_string(),
            iss: "https://visas.example.org".to_string(),
            iat: 1_700_000_000,
            exp: 1_700_003_600,
            jti: "visa-jti-001".to_string(),
            ga4gh_visa_v1: VisaClaim {
                r#type: VisaType::ControlledAccessGrants,
                asserted: 1_699_999_000,
                value: "dataset-a".to_string(),
                source: "https://visas.example.org".to_string(),
                by: None,
                conditions: None,
            },
            scope: None,
            jku: None,
        };
        let json = serde_json::to_value(&claims).expect("serialize");
        assert!(json.get("scope").is_none());
        assert!(json.get("jku").is_none());
    }
}
