// SPDX-License-Identifier: Apache-2.0

//! GA4GH Passport types.

use serde::{Deserialize, Serialize};

/// Standard JWT claims for a GA4GH Passport, including the `ga4gh_passport_v1` claim.
///
/// The `ga4gh_passport_v1` claim value is an array of Visa JWTs encoded as
/// JWS Compact Serialization strings, per the GA4GH Passport specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassportClaims {
    /// Subject identifier for the researcher.
    pub sub: String,
    /// Issuer URL of the Passport broker.
    pub iss: String,
    /// Issued-at timestamp (seconds since Unix epoch).
    pub iat: i64,
    /// Expiration timestamp (seconds since Unix epoch).
    pub exp: i64,
    /// Unique token identifier.
    pub jti: String,
    /// Array of embedded Visa JWT compact-serialization strings.
    #[serde(rename = "ga4gh_passport_v1")]
    pub ga4gh_passport_v1: Vec<String>,
    /// OAuth scope claim, when present on the Passport JWT.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Audience claim, when present on the Passport JWT.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
}

/// A decoded GA4GH Passport wrapping the `ga4gh_passport_v1` visa JWT array.
///
/// This type represents the validated semantic content of a Passport JWT after
/// signature verification (performed by a clearinghouse or equivalent validator).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "PassportClaims", into = "PassportClaims")]
pub struct Passport {
    /// Subject identifier for the researcher.
    pub sub: String,
    /// Issuer URL of the Passport broker.
    pub iss: String,
    /// Issued-at timestamp (seconds since Unix epoch).
    pub iat: i64,
    /// Expiration timestamp (seconds since Unix epoch).
    pub exp: i64,
    /// Unique token identifier.
    pub jti: String,
    /// Raw compact-serialization Visa JWT strings from `ga4gh_passport_v1`.
    pub visa_jwts: Vec<String>,
    /// OAuth scope claim, when present.
    pub scope: Option<String>,
    /// Audience claim, when present.
    pub aud: Option<String>,
}

impl Passport {
    /// Construct a [`Passport`] from decoded JWT claims.
    pub fn from_claims(claims: PassportClaims) -> Self {
        Self {
            sub: claims.sub,
            iss: claims.iss,
            iat: claims.iat,
            exp: claims.exp,
            jti: claims.jti,
            visa_jwts: claims.ga4gh_passport_v1,
            scope: claims.scope,
            aud: claims.aud,
        }
    }

    /// Convert this passport back into JWT claim form for serialization.
    pub fn into_claims(self) -> PassportClaims {
        PassportClaims {
            sub: self.sub,
            iss: self.iss,
            iat: self.iat,
            exp: self.exp,
            jti: self.jti,
            ga4gh_passport_v1: self.visa_jwts,
            scope: self.scope,
            aud: self.aud,
        }
    }
}

impl From<PassportClaims> for Passport {
    fn from(claims: PassportClaims) -> Self {
        Self::from_claims(claims)
    }
}

impl From<Passport> for PassportClaims {
    fn from(passport: Passport) -> Self {
        passport.into_claims()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passport_round_trip() {
        let passport = Passport {
            sub: "researcher@example.org".to_string(),
            iss: "https://broker.example.org".to_string(),
            iat: 1_700_000_000,
            exp: 1_700_003_600,
            jti: "passport-jti-001".to_string(),
            visa_jwts: vec!["eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJyIn0.sig".to_string()],
            scope: None,
            aud: None,
        };

        let json = serde_json::to_string(&passport).expect("serialize");
        let decoded: Passport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(passport, decoded);
    }

    #[test]
    fn passport_claims_round_trip() {
        let claims = PassportClaims {
            sub: "researcher@example.org".to_string(),
            iss: "https://broker.example.org".to_string(),
            iat: 1_700_000_000,
            exp: 1_700_003_600,
            jti: "passport-jti-001".to_string(),
            ga4gh_passport_v1: vec!["eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJyIn0.sig".to_string()],
            scope: Some("openid ga4gh_passport_v1".to_string()),
            aud: None,
        };

        let json = serde_json::to_string(&claims).expect("serialize");
        let decoded: PassportClaims = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(claims, decoded);

        let passport = Passport::from_claims(decoded.clone());
        let round_trip = PassportClaims::from(passport);
        assert_eq!(claims, round_trip);
    }

    #[test]
    fn passport_json_omits_optional_scope_and_aud() {
        let claims = PassportClaims {
            sub: "researcher@example.org".to_string(),
            iss: "https://broker.example.org".to_string(),
            iat: 1_700_000_000,
            exp: 1_700_003_600,
            jti: "passport-jti-001".to_string(),
            ga4gh_passport_v1: vec![],
            scope: None,
            aud: None,
        };

        let json = serde_json::to_value(&claims).expect("serialize");
        assert!(json.get("scope").is_none());
        assert!(json.get("aud").is_none());
    }

    #[test]
    fn passport_rejects_non_numeric_timestamps() {
        let json = r#"{
            "sub": "researcher@example.org",
            "iss": "https://broker.example.org",
            "iat": "not-a-number",
            "exp": 1700003600,
            "jti": "jti",
            "ga4gh_passport_v1": []
        }"#;
        assert!(serde_json::from_str::<PassportClaims>(json).is_err());
    }
}
