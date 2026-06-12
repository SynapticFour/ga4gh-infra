// SPDX-License-Identifier: Apache-2.0

//! Visa JWT minting from stored assertions.

use std::time::{SystemTime, UNIX_EPOCH};

use ga4gh_types::{VisaClaim, VisaJwtClaims};
use jsonwebtoken::encode;
use tracing::instrument;

use crate::config::RegistryConfig;
use crate::error::RegistryError;
use crate::keys::SigningKeys;
use crate::store::VisaAssertion;

/// Mint a signed GA4GH visa JWT from an unsigned stored assertion.
#[instrument(skip(keys, config, assertion))]
pub fn mint_visa_jwt(
    assertion: &VisaAssertion,
    config: &RegistryConfig,
    keys: &SigningKeys,
) -> Result<String, RegistryError> {
    let now = unix_now();
    let lifetime = config.signing.visa_lifetime_seconds as i64;
    let exp = assertion
        .expires_at
        .unwrap_or(now + lifetime)
        .min(now + lifetime);

    let claims = VisaJwtClaims {
        sub: assertion.sub.clone(),
        iss: config.issuer_url().to_string(),
        iat: now,
        exp,
        jti: assertion.id.to_string(),
        ga4gh_visa_v1: VisaClaim {
            r#type: assertion.visa_type.clone(),
            asserted: assertion.asserted,
            value: assertion.value.clone(),
            source: assertion.source.clone(),
            by: assertion.by,
            conditions: assertion.conditions.clone(),
        },
        scope: None,
        jku: Some(config.jwks_url()),
    };

    encode(&keys.signing_header(), &claims, keys.encoding_key())
        .map_err(|err| RegistryError::Signing(err.to_string()))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use ga4gh_types::{VisaAuthority, VisaType};
    use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
    use uuid::Uuid;

    use super::*;
    use crate::config::{AuthConfig, DatabaseConfig, RegistryConfig, ServerConfig, SigningConfig};
    use crate::store::VisaAssertion;
    use crate::test_support::test_signing_keys;

    fn test_config() -> RegistryConfig {
        RegistryConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8081,
                external_url: "https://visas.example.org".to_string(),
                environment: "test".to_string(),
            },
            signing: SigningConfig {
                private_key_pem: "/unused".to_string(),
                visa_lifetime_seconds: 3600,
            },
            database: DatabaseConfig {
                driver: crate::config::DatabaseDriver::Postgres,
                url: None,
                url_env: "REGISTRY_DATABASE_URL".to_string(),
                auto_migrate: false,
            },
            auth: AuthConfig {
                bootstrap_api_key_env: "REGISTRY_BOOTSTRAP_API_KEY".to_string(),
            },
        }
    }

    #[test]
    fn mints_decodeable_visa_jwt() {
        let config = test_config();
        let keys = test_signing_keys();
        let assertion = VisaAssertion {
            id: Uuid::new_v4(),
            sub: "researcher@example.org".to_string(),
            visa_type: VisaType::ControlledAccessGrants,
            value: "dataset-abc".to_string(),
            source: "https://dac.example.org".to_string(),
            by: Some(VisaAuthority::Dac),
            conditions: None,
            asserted: unix_now() - 60,
            created_at: unix_now(),
            revoked_at: None,
            expires_at: None,
        };

        let jwt = mint_visa_jwt(&assertion, &config, keys).expect("mint");
        let header = decode_header(&jwt).expect("header");
        assert_eq!(header.alg, Algorithm::RS256);

        let public_pem = keys.jwks()["keys"][0]["n"].as_str().expect("n");
        let _ = public_pem;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[config.issuer_url()]);
        validation.validate_aud = false;

        let decoded: VisaJwtClaims = decode(
            &jwt,
            &DecodingKey::from_rsa_components(
                keys.jwks()["keys"][0]["n"].as_str().expect("n"),
                keys.jwks()["keys"][0]["e"].as_str().expect("e"),
            )
            .expect("decoding key"),
            &validation,
        )
        .expect("decode")
        .claims;

        assert_eq!(decoded.sub, assertion.sub);
        assert_eq!(decoded.ga4gh_visa_v1.value, "dataset-abc");
        assert_eq!(decoded.jku.as_deref(), Some(config.jwks_url().as_str()));
    }
}
