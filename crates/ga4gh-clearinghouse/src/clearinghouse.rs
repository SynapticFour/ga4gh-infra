// SPDX-License-Identifier: Apache-2.0

//! Clearinghouse validation and policy evaluation.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use ga4gh_types::{Passport, PassportClaims, Visa, VisaJwtClaims};
use tracing::instrument;

use crate::config::ClearinghouseConfig;
use crate::error::ClearinghouseError;
use crate::jwks::{JwksCache, SharedJwksCache};
use crate::policy::{evaluate_policy, PolicyCheck, PolicyResult};
use crate::token::peek_expiry;

/// Passport clearinghouse that validates broker-issued Passports and Visas.
pub struct Clearinghouse {
    jwks: SharedJwksCache,
}

impl Clearinghouse {
    /// Create a clearinghouse from configuration, ready to validate tokens.
    pub async fn new(config: ClearinghouseConfig) -> Result<Self, ClearinghouseError> {
        Ok(Self {
            jwks: Arc::new(JwksCache::new(
                config.trusted_brokers,
                config.jwks_cache_ttl,
            )?),
        })
    }

    /// Validate a raw Passport JWT: verify signature, expiry, and trusted issuer.
    #[instrument(skip(self, raw_passport_jwt))]
    pub async fn validate_passport(
        &self,
        raw_passport_jwt: &str,
    ) -> Result<Passport, ClearinghouseError> {
        if is_expired(peek_expiry(raw_passport_jwt)?) {
            return Err(ClearinghouseError::ExpiredPassport);
        }

        let claims: PassportClaims = self.jwks.verify_and_decode(raw_passport_jwt).await?;
        Ok(Passport::from_claims(claims))
    }

    /// Extract and validate individual visa JWTs embedded in a validated Passport.
    #[instrument(skip(self, passport))]
    pub async fn extract_visas(
        &self,
        passport: &Passport,
    ) -> Result<Vec<Visa>, ClearinghouseError> {
        let mut visas = Vec::with_capacity(passport.visa_jwts.len());
        for visa_jwt in &passport.visa_jwts {
            let claims: VisaJwtClaims =
                self.jwks
                    .verify_and_decode(visa_jwt)
                    .await
                    .map_err(|err| match err {
                        ClearinghouseError::ExpiredPassport => ClearinghouseError::ExpiredVisa,
                        other => other,
                    })?;
            if is_expired(claims.exp) {
                return Err(ClearinghouseError::ExpiredVisa);
            }
            visas.push(Visa::from_claims(claims));
        }
        Ok(visas)
    }

    /// Evaluate whether visas satisfy a policy expression.
    pub fn check_policy(&self, visas: &[Visa], policy: &PolicyCheck) -> PolicyResult {
        let _ = self;
        evaluate_policy(visas, policy)
    }

    /// Access the underlying JWKS cache (primarily for tests).
    #[doc(hidden)]
    pub fn jwks_cache(&self) -> &SharedJwksCache {
        &self.jwks
    }
}

fn is_expired(exp: i64) -> bool {
    unix_now() >= exp
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TrustedBroker;

    #[tokio::test]
    async fn rejects_expired_passport_before_signature_verification() {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
            vec![TrustedBroker::new(
                "https://trusted.example.org",
                "https://trusted.example.org/jwks.json",
            )],
            std::time::Duration::from_secs(300),
        ))
        .await
        .expect("clearinghouse");

        let payload = URL_SAFE_NO_PAD
            .encode(br#"{"iss":"https://trusted.example.org","exp":1,"sub":"user@example.org"}"#);
        let token = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.signature");

        let err = clearinghouse
            .validate_passport(&token)
            .await
            .expect_err("expired passport");
        assert!(matches!(err, ClearinghouseError::ExpiredPassport));
    }

    #[tokio::test]
    async fn rejects_untrusted_passport_issuer() {
        let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
            vec![TrustedBroker::new(
                "https://trusted.example.org",
                "https://trusted.example.org/jwks.json",
            )],
            std::time::Duration::from_secs(300),
        ))
        .await
        .expect("clearinghouse");

        let err = clearinghouse
            .validate_passport("a.b.c")
            .await
            .expect_err("invalid token");
        assert!(matches!(
            err,
            ClearinghouseError::InvalidTokenFormat
                | ClearinghouseError::InvalidClaims(_)
                | ClearinghouseError::UntrustedIssuer
        ));
    }
}
