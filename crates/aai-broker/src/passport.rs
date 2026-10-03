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
    /// Compact JWS.
    pub jwt: String,
    /// JWT `jti`.
    pub jti: String,
    /// JWT `exp` (unix seconds).
    pub exp: i64,
}

/// Mint a GA4GH Passport JWT for the given identity and visa JWT strings.
///
/// `policy` and `verifier` decide the flat claim, `aud`, and which visa strings
/// are embedded. The default policy keeps today's bytes: upstream groups, no
/// `aud`, and the visa strings unchanged.
pub fn mint_passport_jwt(
    keys: &SigningKeys,
    issuer: &str,
    identity: &ResearcherIdentity,
    visa_jwts: &[String],
    lifetime_seconds: u64,
    policy: &TokenClaimsConfig,
    verifier: Option<&VisaVerifier>,
) -> Result<MintedPassport, BrokerError> {
    let now = unix_now();
    let jti = Uuid::new_v4().to_string();
    let exp = now + lifetime_seconds as i64;
    let embedded = embedded_visas(policy, verifier, &identity.sub, visa_jwts);
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
    Ok(MintedPassport { jwt, jti, exp })
}

fn embedded_visas(
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

    #[test]
    fn mints_passport_with_visa_array() {
        let keys = test_signing_keys();
        let identity = ResearcherIdentity {
            sub: "researcher@example.org".to_string(),
            email: None,
            display_name: None,
            affiliation: None,
            groups: vec![],
        };
        let minted = mint_passport_jwt(
            keys,
            "https://broker.example.org",
            &identity,
            &["visa-jwt-one".to_string()],
            3600,
            &TokenClaimsConfig::default(),
            None,
        )
        .expect("mint passport");

        assert_eq!(minted.jwt.matches('.').count(), 2);
        assert!(!minted.jti.is_empty());
    }
}
