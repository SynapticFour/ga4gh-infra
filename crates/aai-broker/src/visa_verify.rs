// SPDX-License-Identifier: Apache-2.0

//! RS256 checks for visa JWTs the broker itself fetched.
//!
//! The verifier uses only a configured JWKS. It does not read `jku` and it does
//! not fetch a key set named by the visa.

use std::fs;

use ga4gh_types::VisaJwtClaims;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::Deserialize;

use crate::error::BrokerError;

const JWKS_BYTE_LIMIT: usize = 1024 * 1024;

/// Visa-issuer public keys loaded at startup.
pub struct VisaVerifier {
    keys: Vec<VisaKey>,
}

struct VisaKey {
    kid: Option<String>,
    decoding: DecodingKey,
}

#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<JwkDocument>,
}

#[derive(Debug, Deserialize)]
struct JwkDocument {
    kty: String,
    kid: Option<String>,
    alg: Option<String>,
    #[serde(rename = "use")]
    use_: Option<String>,
    n: Option<String>,
    e: Option<String>,
}

impl VisaVerifier {
    /// Load a JWKS from a file or a URL. Exactly one source is accepted.
    pub async fn load(
        jwks_file: Option<&str>,
        jwks_url: Option<&str>,
        http: &Client,
    ) -> Result<Self, BrokerError> {
        match (jwks_file, jwks_url) {
            (Some(path), None) => {
                let bytes = fs::read(path).map_err(|err| {
                    BrokerError::Config(format!("reading visa jwks {path}: {err}"))
                })?;
                Self::from_jwks_bytes(&bytes)
            }
            (None, Some(url)) => {
                let response =
                    http.get(url).send().await.map_err(|err| {
                        BrokerError::Config(format!("visa jwks fetch {url}: {err}"))
                    })?;
                let status = response.status();
                if !status.is_success() {
                    return Err(BrokerError::Config(format!(
                        "visa jwks fetch {url}: HTTP {status}"
                    )));
                }
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|err| BrokerError::Config(format!("visa jwks fetch {url}: {err}")))?;
                Self::from_jwks_bytes(&bytes)
            }
            _ => Err(BrokerError::Config(
                "token_claims requires jwks_file or jwks_url".to_string(),
            )),
        }
    }

    /// Parse a JWKS document. Only RSA keys usable for RS256 are kept.
    pub fn from_jwks_json(json: &str) -> Result<Self, BrokerError> {
        Self::from_jwks_bytes(json.as_bytes())
    }

    fn from_jwks_bytes(bytes: &[u8]) -> Result<Self, BrokerError> {
        if bytes.len() > JWKS_BYTE_LIMIT {
            return Err(BrokerError::Config(
                "visa jwks is larger than 1 MiB".to_string(),
            ));
        }
        let document: JwksDocument = serde_json::from_slice(bytes)
            .map_err(|err| BrokerError::Config(format!("visa jwks: {err}")))?;
        let mut keys = Vec::new();
        for jwk in document.keys {
            if jwk.kty != "RSA" {
                continue;
            }
            if jwk.alg.as_deref().is_some_and(|alg| alg != "RS256") {
                continue;
            }
            if jwk.use_.as_deref().is_some_and(|use_| use_ != "sig") {
                continue;
            }
            let (Some(modulus), Some(exponent)) = (jwk.n.as_deref(), jwk.e.as_deref()) else {
                continue;
            };
            let decoding = DecodingKey::from_rsa_components(modulus, exponent)
                .map_err(|err| BrokerError::Config(format!("visa jwks rsa key: {err}")))?;
            keys.push(VisaKey {
                kid: jwk.kid,
                decoding,
            });
        }
        if keys.is_empty() {
            return Err(BrokerError::Config(
                "visa jwks has no RS256 keys".to_string(),
            ));
        }
        Ok(Self { keys })
    }

    /// Check one visa JWT. `jku` on the header is ignored.
    ///
    /// The visa `sub` must equal `expected_sub`. A failed check returns an error
    /// and contributes nothing to the passport.
    pub fn verify(&self, token: &str, expected_sub: &str) -> Result<VisaJwtClaims, BrokerError> {
        let header = decode_header(token)
            .map_err(|err| BrokerError::Config(format!("visa header: {err}")))?;
        if header.alg != Algorithm::RS256 {
            return Err(BrokerError::Config("visa jwt must use RS256".to_string()));
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_aud = false;
        let candidates: Vec<&VisaKey> = if let Some(kid) = header.kid.as_deref() {
            self.keys
                .iter()
                .filter(|key| key.kid.as_deref() == Some(kid))
                .collect()
        } else {
            self.keys.iter().collect()
        };
        if candidates.is_empty() {
            return Err(BrokerError::Config(
                "visa jwt kid is not in the configured jwks".to_string(),
            ));
        }
        let mut last_error = None;
        for key in candidates {
            match decode::<VisaJwtClaims>(token, &key.decoding, &validation) {
                Ok(data) => {
                    if data.claims.sub != expected_sub {
                        return Err(BrokerError::Config(
                            "visa sub does not match the passport subject".to_string(),
                        ));
                    }
                    return Ok(data.claims);
                }
                Err(err) => last_error = Some(err),
            }
        }
        Err(BrokerError::Config(format!(
            "visa signature check failed: {}",
            last_error
                .map(|err| err.to_string())
                .unwrap_or_else(|| "no key".to_string())
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use ga4gh_types::{VisaClaim, VisaJwtClaims, VisaType};
    use jsonwebtoken::{decode, encode, Algorithm, Validation};
    use rand::SeedableRng;
    use rsa::pkcs8::EncodePrivateKey;
    use rsa::RsaPrivateKey;
    use serde_json::Value;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::config::TokenClaimsConfig;
    use crate::identity::ResearcherIdentity;
    use crate::keys::SigningKeys;
    use crate::passport::mint_passport_jwt;
    use crate::session::unix_now;
    use crate::test_support::test_signing_keys;

    fn visa_issuer() -> &'static SigningKeys {
        static KEYS: OnceLock<SigningKeys> = OnceLock::new();
        KEYS.get_or_init(|| {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
            let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate visa issuer key");
            let pem = private_key
                .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
                .expect("encode visa issuer key")
                .to_string();
            SigningKeys::from_pem(&pem).expect("visa issuer keys")
        })
    }

    fn other_issuer() -> &'static SigningKeys {
        static KEYS: OnceLock<SigningKeys> = OnceLock::new();
        KEYS.get_or_init(|| {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(9);
            let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate other key");
            let pem = private_key
                .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
                .expect("encode other key")
                .to_string();
            SigningKeys::from_pem(&pem).expect("other keys")
        })
    }

    fn verifier() -> VisaVerifier {
        VisaVerifier::from_jwks_json(&visa_issuer().jwks().to_string()).expect("verifier")
    }

    fn sign_visa(keys: &SigningKeys, sub: &str, visa_type: &str, value: &str, exp: i64) -> String {
        let now = unix_now();
        let claims = VisaJwtClaims {
            sub: sub.to_string(),
            iss: "https://visa.example.org".to_string(),
            iat: now,
            exp,
            jti: format!("jti-{value}"),
            ga4gh_visa_v1: VisaClaim {
                r#type: visa_type.parse::<VisaType>().expect("visa type"),
                asserted: now,
                value: value.to_string(),
                source: "https://visa.example.org".to_string(),
                by: None,
                conditions: None,
            },
            scope: Some("openid".to_string()),
            jku: Some("https://attacker.example/jwks.json".to_string()),
        };
        let mut header = keys.signing_header();
        header.jku = Some("https://attacker.example/jwks.json".to_string());
        encode(&header, &claims, keys.encoding_key()).expect("sign visa")
    }

    fn identity() -> ResearcherIdentity {
        ResearcherIdentity {
            sub: "researcher@example.org".to_string(),
            email: Some("researcher@example.org".to_string()),
            display_name: Some("Researcher".to_string()),
            affiliation: None,
            groups: vec!["from-idp".to_string()],
        }
    }

    fn payload(jwt: &str) -> Value {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_aud = false;
        decode::<Value>(jwt, test_signing_keys().decoding_key(), &validation)
            .expect("decode passport")
            .claims
    }

    fn enabled_policy() -> TokenClaimsConfig {
        TokenClaimsConfig {
            enabled: true,
            jwks_file: Some("/unused.json".to_string()),
            ..TokenClaimsConfig::default()
        }
    }

    #[test]
    fn feature_off_keeps_upstream_groups_and_unverified_visas() {
        let forged = "not-a-jwt".to_string();
        let minted = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &[forged],
            3600,
            &TokenClaimsConfig::default(),
            None,
        )
        .expect("mint");
        let claims = payload(&minted.jwt);
        assert_eq!(claims["groups"][0], "from-idp");
        assert!(claims.get("aud").is_none());
        assert_eq!(claims["ga4gh_passport_v1"][0], "not-a-jwt");
        assert_eq!(claims["email"], "researcher@example.org");
        assert_eq!(claims["name"], "Researcher");
    }

    #[test]
    fn feature_on_fills_groups_only_from_checked_affiliation_visas() {
        let subject = "researcher@example.org";
        let exp = unix_now() + 3600;
        let role = sign_visa(
            visa_issuer(),
            subject,
            "AffiliationAndRole",
            "data-steward@demo.invalid",
            exp,
        );
        let other_type = sign_visa(visa_issuer(), subject, "ResearcherStatus", "bona-fide", exp);
        let forged = sign_visa(
            other_issuer(),
            subject,
            "AffiliationAndRole",
            "forged@demo.invalid",
            exp,
        );
        let mut policy = enabled_policy();
        policy.audiences = vec!["https://resources.example".to_string()];
        let minted = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &[role, other_type, forged.clone()],
            3600,
            &policy,
            Some(&verifier()),
        )
        .expect("mint");
        let claims = payload(&minted.jwt);
        assert_eq!(claims["groups"].as_array().expect("groups").len(), 1);
        assert_eq!(claims["groups"][0], "data-steward@demo.invalid");
        assert_eq!(claims["aud"], "https://resources.example");
        assert!(claims["aud"].is_string());
        assert_eq!(claims["sub"], subject);
        assert_eq!(claims["email"], "researcher@example.org");
        let embedded = claims["ga4gh_passport_v1"].as_array().expect("visas");
        assert_eq!(embedded.len(), 3);
        assert_eq!(embedded[2], forged);
    }

    #[test]
    fn empty_audience_omits_aud_and_a_missing_verifier_leaves_the_claim_empty() {
        let minted = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &["visa".to_string()],
            3600,
            &enabled_policy(),
            None,
        )
        .expect("mint");
        let claims = payload(&minted.jwt);
        assert!(claims.get("groups").is_none());
        assert!(claims.get("aud").is_none());
    }

    #[test]
    fn verify_embedded_drops_bad_visas_and_off_keeps_them() {
        let subject = "researcher@example.org";
        let exp = unix_now() + 3600;
        let good = sign_visa(
            visa_issuer(),
            subject,
            "AffiliationAndRole",
            "role@demo.invalid",
            exp,
        );
        let bad = sign_visa(
            other_issuer(),
            subject,
            "AffiliationAndRole",
            "nope@demo.invalid",
            exp,
        );
        let mut checked = enabled_policy();
        checked.verify_embedded_visas = true;
        let dropped = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &[good.clone(), bad.clone()],
            3600,
            &checked,
            Some(&verifier()),
        )
        .expect("mint");
        let claims = payload(&dropped.jwt);
        assert_eq!(
            claims["ga4gh_passport_v1"].as_array().expect("visas").len(),
            1
        );
        assert_eq!(claims["groups"][0], "role@demo.invalid");

        let kept = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &[good, bad.clone()],
            3600,
            &enabled_policy(),
            Some(&verifier()),
        )
        .expect("mint");
        let kept_claims = payload(&kept.jwt);
        assert_eq!(
            kept_claims["ga4gh_passport_v1"]
                .as_array()
                .expect("visas")
                .len(),
            2
        );
        assert_eq!(kept_claims["groups"].as_array().expect("groups").len(), 1);

        let mut no_verifier = enabled_policy();
        no_verifier.verify_embedded_visas = true;
        let empty = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &[bad],
            3600,
            &no_verifier,
            None,
        )
        .expect("mint");
        let empty_claims = payload(&empty.jwt);
        assert!(empty_claims["ga4gh_passport_v1"]
            .as_array()
            .expect("visas")
            .is_empty());
    }

    #[test]
    fn jku_is_ignored_and_the_subject_must_match() {
        let exp = unix_now() + 3600;
        let signed = sign_visa(
            visa_issuer(),
            "researcher@example.org",
            "AffiliationAndRole",
            "role@demo.invalid",
            exp,
        );
        let header = decode_header(&signed).expect("header");
        assert_eq!(
            header.jku.as_deref(),
            Some("https://attacker.example/jwks.json")
        );
        assert!(verifier().verify(&signed, "researcher@example.org").is_ok());
        assert!(verifier().verify(&signed, "other@example.org").is_err());
        let expired = sign_visa(
            visa_issuer(),
            "researcher@example.org",
            "AffiliationAndRole",
            "old@demo.invalid",
            unix_now() - 120,
        );
        assert!(verifier()
            .verify(&expired, "researcher@example.org")
            .is_err());
        let attacker = sign_visa(
            other_issuer(),
            "researcher@example.org",
            "AffiliationAndRole",
            "forged@demo.invalid",
            exp,
        );
        assert!(verifier()
            .verify(&attacker, "researcher@example.org")
            .is_err());
    }

    #[test]
    fn two_audiences_refuse_at_mint_as_well_as_config() {
        let mut policy = enabled_policy();
        policy.audiences = vec![
            "https://a.example".to_string(),
            "https://b.example".to_string(),
        ];
        let err = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &[],
            3600,
            &policy,
            Some(&verifier()),
        );
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn jwks_url_down_fails_and_a_live_url_loads() {
        let down = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&down)
            .await;
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("client");
        let url = format!("{}/jwks.json", down.uri());
        assert!(VisaVerifier::load(None, Some(&url), &client).await.is_err());

        let up = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(visa_issuer().jwks().to_string()),
            )
            .mount(&up)
            .await;
        let loaded = VisaVerifier::load(None, Some(&format!("{}/jwks.json", up.uri())), &client)
            .await
            .expect("load");
        let exp = unix_now() + 3600;
        let signed = sign_visa(
            visa_issuer(),
            "researcher@example.org",
            "AffiliationAndRole",
            "role@demo.invalid",
            exp,
        );
        assert!(loaded.verify(&signed, "researcher@example.org").is_ok());
    }

    #[test]
    fn custom_claim_name_is_not_groups() {
        let exp = unix_now() + 3600;
        let role = sign_visa(
            visa_issuer(),
            "researcher@example.org",
            "AffiliationAndRole",
            "role@demo.invalid",
            exp,
        );
        let mut policy = enabled_policy();
        policy.claim_name = "entitlements".to_string();
        let minted = mint_passport_jwt(
            test_signing_keys(),
            "https://broker.example.org",
            &identity(),
            &[role],
            3600,
            &policy,
            Some(&verifier()),
        )
        .expect("mint");
        let claims = payload(&minted.jwt);
        assert!(claims.get("groups").is_none());
        assert_eq!(claims["entitlements"][0], "role@demo.invalid");
    }
}
