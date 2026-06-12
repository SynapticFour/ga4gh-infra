// SPDX-License-Identifier: Apache-2.0

//! JWT parsing helpers used before signature verification.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{decode_header, Header};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::error::ClearinghouseError;

/// Decode a JWT header without verifying the signature.
pub fn decode_jwt_header(token: &str) -> Result<Header, ClearinghouseError> {
    decode_header(token).map_err(|err| ClearinghouseError::InvalidClaims(err.to_string()))
}

/// Read the `iss` claim from a JWT payload without verifying the signature.
pub fn peek_issuer(token: &str) -> Result<String, ClearinghouseError> {
    #[derive(Deserialize)]
    struct IssuerClaim {
        iss: String,
    }

    Ok(peek_claims::<IssuerClaim>(token)?.iss)
}

/// Read the `exp` claim from a JWT payload without verifying the signature.
pub fn peek_expiry(token: &str) -> Result<i64, ClearinghouseError> {
    #[derive(Deserialize)]
    struct ExpiryClaim {
        exp: i64,
    }

    Ok(peek_claims::<ExpiryClaim>(token)?.exp)
}

/// Deserialize JWT payload JSON without verifying the signature.
pub fn peek_claims<T>(token: &str) -> Result<T, ClearinghouseError>
where
    T: DeserializeOwned,
{
    let payload = peek_json_payload(token)?;
    serde_json::from_value(payload)
        .map_err(|err| ClearinghouseError::InvalidClaims(err.to_string()))
}

fn peek_json_payload(token: &str) -> Result<Value, ClearinghouseError> {
    let payload_segment = token
        .split('.')
        .nth(1)
        .ok_or(ClearinghouseError::InvalidTokenFormat)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload_segment)
        .map_err(|_| ClearinghouseError::InvalidTokenFormat)?;
    serde_json::from_slice(&bytes).map_err(|err| ClearinghouseError::InvalidClaims(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    #[test]
    fn rejects_malformed_token() {
        assert!(matches!(
            peek_issuer("not-a-jwt"),
            Err(ClearinghouseError::InvalidTokenFormat)
        ));
    }

    #[test]
    fn peeks_issuer_expiry_and_claims_without_verification() {
        let payload = URL_SAFE_NO_PAD.encode(
            br#"{"iss":"https://broker.example.org","exp":1700000000,"sub":"user@example.org"}"#,
        );
        let token = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.signature");

        assert_eq!(
            peek_issuer(&token).expect("issuer"),
            "https://broker.example.org"
        );
        assert_eq!(peek_expiry(&token).expect("expiry"), 1_700_000_000);

        let claims: serde_json::Value = peek_claims(&token).expect("claims");
        assert_eq!(claims["sub"], "user@example.org");
    }
}
