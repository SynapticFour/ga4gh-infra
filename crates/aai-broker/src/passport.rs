// SPDX-License-Identifier: Apache-2.0

//! Passport JWT minting using broker signing keys.

use ga4gh_types::PassportClaims;
use jsonwebtoken::encode;
use uuid::Uuid;

use crate::error::BrokerError;
use crate::identity::ResearcherIdentity;
use crate::keys::SigningKeys;
use crate::session::unix_now;

/// Mint a GA4GH Passport JWT for the given identity and visa JWT strings.
pub fn mint_passport_jwt(
    keys: &SigningKeys,
    issuer: &str,
    identity: &ResearcherIdentity,
    visa_jwts: &[String],
    lifetime_seconds: u64,
) -> Result<String, BrokerError> {
    let now = unix_now();
    let claims = PassportClaims {
        sub: identity.sub.clone(),
        iss: issuer.to_string(),
        iat: now,
        exp: now + lifetime_seconds as i64,
        jti: Uuid::new_v4().to_string(),
        ga4gh_passport_v1: visa_jwts.to_vec(),
        scope: Some("openid ga4gh_passport_v1".to_string()),
        aud: None,
    };

    encode(&keys.signing_header(), &claims, keys.encoding_key())
        .map_err(|err| BrokerError::Signing(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_signing_keys;

    #[test]
    fn mints_passport_with_visa_array() {
        let keys = test_signing_keys();
        let identity = ResearcherIdentity {
            sub: "researcher@example.org".to_string(),
            email: None,
            affiliation: None,
            extra: Default::default(),
        };
        let token = mint_passport_jwt(
            keys,
            "https://broker.example.org",
            &identity,
            &["visa-jwt-one".to_string()],
            3600,
        )
        .expect("mint passport");

        assert_eq!(token.matches('.').count(), 2);
    }
}
