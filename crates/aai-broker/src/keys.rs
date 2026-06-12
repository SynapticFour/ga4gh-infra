// SPDX-License-Identifier: Apache-2.0

//! RS256 signing keys and JWKS export for downstream validation.

use std::fs;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header};
use rsa::pkcs8::{DecodePrivateKey, EncodePublicKey};
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::error::BrokerError;

/// Broker signing material used to mint Passports and expose `/jwks.json`.
pub struct SigningKeys {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    kid: String,
    jwks: Value,
}

impl SigningKeys {
    /// Load an RS256 private key from a PEM file and derive JWKS metadata.
    pub fn from_pem_file(path: &str) -> Result<Self, BrokerError> {
        let pem = fs::read_to_string(path)
            .map_err(|err| BrokerError::Config(format!("reading signing key: {err}")))?;
        Self::from_pem(&pem)
    }

    /// Load an RS256 private key from PEM bytes and derive JWKS metadata.
    pub fn from_pem(pem: &str) -> Result<Self, BrokerError> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(pem)
            .map_err(|err| BrokerError::Config(format!("parsing signing key: {err}")))?;
        let public_key = private_key.to_public_key();

        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
        let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public_key.n().to_bytes_be()));

        let jwks = json!({
            "keys": [{
                "kty": "RSA",
                "kid": kid,
                "use": "sig",
                "alg": "RS256",
                "n": n,
                "e": e,
            }]
        });

        let public_pem = private_key
            .to_public_key()
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .map_err(|err| BrokerError::Config(format!("public key: {err}")))?;

        let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes())
            .map_err(|err| BrokerError::Config(format!("encoding key: {err}")))?;
        let decoding_key = DecodingKey::from_rsa_pem(public_pem.as_bytes())
            .map_err(|err| BrokerError::Config(format!("decoding key: {err}")))?;

        Ok(Self {
            encoding_key,
            decoding_key,
            kid,
            jwks,
        })
    }

    /// JWT header for RS256 tokens issued by this broker.
    pub fn signing_header(&self) -> Header {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        header
    }

    /// Encoding key for signing JWTs.
    pub fn encoding_key(&self) -> &EncodingKey {
        &self.encoding_key
    }

    /// Decoding key for validating broker-issued access tokens.
    pub fn decoding_key(&self) -> &DecodingKey {
        &self.decoding_key
    }

    /// JWKS document served at `/jwks.json`.
    pub fn jwks(&self) -> &Value {
        &self.jwks
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::test_signing_keys;

    #[test]
    fn loads_test_key_and_exports_jwks() {
        let keys = test_signing_keys();
        assert!(keys.jwks()["keys"]
            .as_array()
            .is_some_and(|k| !k.is_empty()));
        assert!(keys.signing_header().kid.is_some());
    }
}
