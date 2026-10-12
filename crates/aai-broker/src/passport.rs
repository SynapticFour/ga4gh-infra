// SPDX-License-Identifier: Apache-2.0

//! Passport JWT minting using broker signing keys.

use ga4gh_types::PassportClaims;
use jsonwebtoken::encode;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::TokenClaimsConfig;
use crate::error::BrokerError;
use crate::identity::ResearcherIdentity;
use crate::keys::SigningKeys;
use crate::session::unix_now;
use crate::visa_verify::VisaVerifier;

/// Minted Passport JWT plus identifiers used by the revocation ledger.
pub struct MintedPassport {
    /// Compact JWS (access token / self-contained Passport depending on embed mode).
    pub jwt: String,
    /// JWT `jti`.
    pub jti: String,
    /// JWT `exp` (unix seconds).
    pub exp: i64,
    /// Visas selected for this Passport after optional embed verification.
    /// When `embed_visas` is false these are omitted from `jwt` and held for `/userinfo`.
    pub visas: Vec<String>,
}

/// Mint a GA4GH Passport JWT for the given identity and visa JWT strings.
///
/// `policy` and `verifier` decide the flat claim, `aud`, and which visa strings
/// are selected. `embed_visas` (ADR-005) decides whether those strings appear in
/// `ga4gh_passport_v1` inside the JWT. The default policy and `embed_visas =
/// true` keep today's bytes: upstream groups, no `aud`, and visas embedded.
#[allow(clippy::too_many_arguments)] // identity + visa set + ADR-004/005 policy knobs
pub fn mint_passport_jwt(
    keys: &SigningKeys,
    issuer: &str,
    identity: &ResearcherIdentity,
    visa_jwts: &[String],
    lifetime_seconds: u64,
    policy: &TokenClaimsConfig,
    verifier: Option<&VisaVerifier>,
    embed_visas: bool,
) -> Result<MintedPassport, BrokerError> {
    let now = unix_now();
    let jti = Uuid::new_v4().to_string();
    let exp = now + lifetime_seconds as i64;
    let selected = selected_visas(policy, verifier, &identity.sub, visa_jwts);
    let embedded = if embed_visas {
        selected.clone()
    } else {
        Vec::new()
    };
    let flat = flat_claim_values(policy, verifier, identity, visa_jwts);
    let aud = audience(policy)?;
    let groups = if !policy.enabled || policy.claim_name == "groups" {
        flat.clone()
    } else {
        Vec::new()
    };
    let mut claims = serde_json::to_value(PassportClaims {
        sub: identity.sub.clone(),
        iss: issuer.to_string(),
        iat: now,
        exp,
        jti: jti.clone(),
        ga4gh_passport_v1: embedded,
        scope: Some("openid ga4gh_passport_v1".to_string()),
        aud,
        email: identity.email.clone(),
        name: identity.display_name.clone(),
        groups,
    })
    .map_err(|err| BrokerError::Signing(err.to_string()))?;
    if policy.enabled && policy.claim_name != "groups" && !flat.is_empty() {
        if let Value::Object(map) = &mut claims {
            map.insert(policy.claim_name.clone(), json!(flat));
        }
    }

    let jwt = encode(&keys.signing_header(), &claims, keys.encoding_key())
        .map_err(|err| BrokerError::Signing(err.to_string()))?;
    Ok(MintedPassport {
        jwt,
        jti,
        exp,
        visas: selected,
    })
}

fn selected_visas(
    policy: &TokenClaimsConfig,
    verifier: Option<&VisaVerifier>,
    subject: &str,
    visa_jwts: &[String],
) -> Vec<String> {
    if !policy.verify_embedded_visas {
        return visa_jwts.to_vec();
    }
    let Some(verifier) = verifier else {
        return Vec::new();
    };
    visa_jwts
        .iter()
        .filter(|jwt| verifier.verify(jwt, subject).is_ok())
        .cloned()
        .collect()
}

fn flat_claim_values(
    policy: &TokenClaimsConfig,
    verifier: Option<&VisaVerifier>,
    identity: &ResearcherIdentity,
    visa_jwts: &[String],
) -> Vec<String> {
    if !policy.enabled {
        return identity.groups.clone();
    }
    let Some(verifier) = verifier else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for jwt in visa_jwts {
        let Ok(claims) = verifier.verify(jwt, &identity.sub) else {
            continue;
        };
        if claims.ga4gh_visa_v1.r#type.as_str() != policy.visa_type {
            continue;
        }
        if !values.contains(&claims.ga4gh_visa_v1.value) {
            values.push(claims.ga4gh_visa_v1.value);
        }
    }
    values
}

fn audience(policy: &TokenClaimsConfig) -> Result<Option<String>, BrokerError> {
    if !policy.enabled {
        return Ok(None);
    }
    match policy.audiences.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(one.clone())),
        _ => Err(BrokerError::Config(
            "token_claims.audiences accepts at most one audience".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TokenClaimsConfig;
    use crate::test_support::test_signing_keys;
    use jsonwebtoken::{decode, Algorithm, Validation};
    use serde_json::Value;

    fn identity() -> ResearcherIdentity {
        ResearcherIdentity {
            sub: "researcher@example.org".to_string(),
            email: None,
            display_name: None,
            affiliation: None,
            groups: vec![],
        }
    }

    fn decode_claims(jwt: &str) -> Value {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&["https://broker.example.org"]);
        validation.validate_aud = false;
        decode::<Value>(jwt, test_signing_keys().decoding_key(), &validation)
            .expect("decode")
            .claims
    }

    #[test]
    fn mints_passport_with_visa_array() {
        let keys = test_signing_keys();
        let minted = mint_passport_jwt(
            keys,
            "https://broker.example.org",
            &identity(),
            &["visa-jwt-one".to_string()],
            3600,
            &TokenClaimsConfig::default(),
            None,
            true,
        )
        .expect("mint passport");

        assert_eq!(minted.jwt.matches('.').count(), 2);
        assert!(!minted.jti.is_empty());
        assert_eq!(minted.visas, vec!["visa-jwt-one".to_string()]);
        let claims = decode_claims(&minted.jwt);
        assert_eq!(claims["ga4gh_passport_v1"][0], "visa-jwt-one");
    }

    #[test]
    fn embed_visas_false_omits_visas_from_jwt_but_returns_them() {
        let visas: Vec<String> = (0..22)
            .map(|i| format!("visa-jwt-{i}-{}", "x".repeat(80)))
            .collect();
        let minted = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &visas,
            3600,
            &TokenClaimsConfig::default(),
            None,
            false,
        )
        .expect("mint");
        let claims = decode_claims(&minted.jwt);
        assert!(claims["ga4gh_passport_v1"]
            .as_array()
            .expect("array")
            .is_empty());
        assert_eq!(minted.visas.len(), 22);
        // Passport-scoped access token must stay well under typical proxy header budgets.
        assert!(
            minted.jwt.len() < 2_048,
            "access token {} bytes with embed_visas=false",
            minted.jwt.len()
        );
    }

    /// Size regression: 22 RS256-class visa stubs (~900 B each) recreate the showcase
    /// researcher Passport shape. Embedding must stay the default and exceed 16 KiB;
    /// `embed_visas=false` must stay under 2 KiB (ADR-005 / docs/PASSPORT-SIZE.md).
    #[test]
    fn showcase_scale_embed_vs_userinfo_access_token_size() {
        let visas: Vec<String> = (0..22)
            .map(|i| {
                // ~900 char stand-in for a RS256-2048 visa JWT (hdr+claims+sig).
                format!("eyJhbGciOiJSUzI1NiJ9.{i}.{}", "s".repeat(860))
            })
            .collect();
        let sum_visas: usize = visas.iter().map(|v| v.len()).sum();
        assert!(
            sum_visas > 18_000,
            "fixture visas too small ({sum_visas}); adjust stub length"
        );

        let embedded = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &visas,
            900,
            &TokenClaimsConfig::default(),
            None,
            true,
        )
        .expect("embed on");
        let auth_embedded = format!("Bearer {}", embedded.jwt);
        assert!(
            auth_embedded.len() > 16_384,
            "embedded Authorization should exceed 16 KiB (got {})",
            auth_embedded.len()
        );
        // Caddy showcase sets 64 KiB MaxHeaderBytes; leave headroom for other headers.
        assert!(
            auth_embedded.len() < 64 * 1024,
            "fixture unexpectedly larger than Caddy budget"
        );

        let compact = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &visas,
            900,
            &TokenClaimsConfig::default(),
            None,
            false,
        )
        .expect("embed off");
        let auth_compact = format!("Bearer {}", compact.jwt);
        assert!(
            auth_compact.len() < 2_048,
            "userinfo-mode Authorization must stay small (got {})",
            auth_compact.len()
        );
        assert_eq!(compact.visas.len(), 22);
        let embedded_claims = decode_claims(&embedded.jwt);
        assert_eq!(
            embedded_claims["ga4gh_passport_v1"]
                .as_array()
                .expect("arr")
                .len(),
            22
        );
    }
}
