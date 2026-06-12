//! Deterministic test issuer resembling ELIXIR/GA4GH passport shapes.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ga4gh_types::{PassportClaims, VisaClaim, VisaJwtClaims, VisaType};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rand::SeedableRng;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde_json::json;
use sha2::{Digest, Sha256};

pub const BROKER_ISSUER: &str = "https://test-broker.example.org";
pub const VISA_ISSUER: &str = "https://test-visas.example.org";

pub struct TestIssuer {
    broker_private_pem: String,
    visa_private_pem: String,
    broker_kid: String,
    visa_kid: String,
    broker_encoding_key: EncodingKey,
    visa_encoding_key: EncodingKey,
}

impl Default for TestIssuer {
    fn default() -> Self {
        Self::new()
    }
}

impl TestIssuer {
    pub fn new() -> Self {
        let broker_key = deterministic_rsa_key(11);
        let visa_key = deterministic_rsa_key(22);
        Self::from_keys(broker_key, visa_key)
    }

    fn from_keys(broker_key: RsaPrivateKey, visa_key: RsaPrivateKey) -> Self {
        let broker_private_pem = broker_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("broker pem")
            .to_string();
        let visa_private_pem = visa_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("visa pem")
            .to_string();

        let broker_public = broker_key.to_public_key();
        let visa_public = visa_key.to_public_key();
        let broker_kid = kid_from_modulus(broker_public.n().to_bytes_be());
        let visa_kid = kid_from_modulus(visa_public.n().to_bytes_be());

        Self {
            broker_encoding_key: EncodingKey::from_rsa_pem(broker_private_pem.as_bytes())
                .expect("broker encoding key"),
            visa_encoding_key: EncodingKey::from_rsa_pem(visa_private_pem.as_bytes())
                .expect("visa encoding key"),
            broker_private_pem,
            visa_private_pem,
            broker_kid,
            visa_kid,
        }
    }

    pub fn broker_jwks_json(&self) -> serde_json::Value {
        jwks_from_pem(&self.broker_private_pem, &self.broker_kid)
    }

    pub fn visa_jwks_json(&self) -> serde_json::Value {
        jwks_from_pem(&self.visa_private_pem, &self.visa_kid)
    }

    pub fn mint_visa_jwt(&self, claim: VisaClaim, subject: &str, jti: &str) -> String {
        self.mint_visa_jwt_with_expiry(claim, subject, jti, unix_now() + 3600)
    }

    pub fn mint_visa_jwt_with_expiry(
        &self,
        claim: VisaClaim,
        subject: &str,
        jti: &str,
        exp: i64,
    ) -> String {
        let now = unix_now();
        let claims = VisaJwtClaims {
            sub: subject.to_string(),
            iss: VISA_ISSUER.to_string(),
            iat: now,
            exp,
            jti: jti.to_string(),
            ga4gh_visa_v1: claim,
            scope: Some("openid".to_string()),
            jku: None,
        };
        encode(
            &self.signing_header(&self.visa_kid),
            &claims,
            &self.visa_encoding_key,
        )
        .expect("encode visa")
    }

    pub fn mint_passport_jwt(&self, subject: &str, visa_jwts: Vec<String>) -> String {
        self.mint_passport_jwt_with_expiry(subject, visa_jwts, unix_now() + 3600)
    }

    pub fn mint_passport_jwt_with_expiry(
        &self,
        subject: &str,
        visa_jwts: Vec<String>,
        exp: i64,
    ) -> String {
        let now = unix_now();
        let claims = PassportClaims {
            sub: subject.to_string(),
            iss: BROKER_ISSUER.to_string(),
            iat: now,
            exp,
            jti: "test-passport-jti".to_string(),
            ga4gh_passport_v1: visa_jwts,
            scope: Some("openid ga4gh_passport_v1".to_string()),
            aud: None,
        };
        encode(
            &self.signing_header(&self.broker_kid),
            &claims,
            &self.broker_encoding_key,
        )
        .expect("encode passport")
    }

    pub fn elixir_shaped_visas(&self, subject: &str) -> Vec<String> {
        vec![
            self.mint_visa_jwt(
                VisaClaim {
                    r#type: VisaType::AffiliationAndRole,
                    asserted: unix_now() - 3600,
                    value: "faculty@uni-heidelberg.de".to_string(),
                    source: "https://login.elixir-europe.org/oidc/".to_string(),
                    by: None,
                    conditions: None,
                },
                subject,
                "visa-affiliation",
            ),
            self.mint_visa_jwt(
                VisaClaim {
                    r#type: VisaType::ResearcherStatus,
                    asserted: unix_now() - 3600,
                    value: "https://doi.org/10.1038/s41431-018-0219-y".to_string(),
                    source: "https://visas.example.org".to_string(),
                    by: None,
                    conditions: None,
                },
                subject,
                "visa-researcher-status",
            ),
            self.mint_visa_jwt(
                VisaClaim {
                    r#type: VisaType::ControlledAccessGrants,
                    asserted: unix_now() - 3600,
                    value: "dataset-registered-access-demo".to_string(),
                    source: "https://www.ebi.ac.uk/ega/dacs/EGAC00000000001".to_string(),
                    by: None,
                    conditions: None,
                },
                subject,
                "visa-controlled-access",
            ),
        ]
    }

    fn signing_header(&self, kid: &str) -> Header {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        header
    }
}

fn deterministic_rsa_key(seed: u64) -> RsaPrivateKey {
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    RsaPrivateKey::new(&mut rng, 2048).expect("rsa key")
}

fn kid_from_modulus(modulus: Vec<u8>) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(modulus))
}

fn jwks_from_pem(private_pem: &str, kid: &str) -> serde_json::Value {
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_pem).expect("parse pem");
    let public_key = private_key.to_public_key();
    json!({
        "keys": [{
            "kty": "RSA",
            "kid": kid,
            "use": "sig",
            "alg": "RS256",
            "n": URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be()),
            "e": URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be()),
        }]
    })
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
