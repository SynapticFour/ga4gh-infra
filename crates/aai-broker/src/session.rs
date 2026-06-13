// SPDX-License-Identifier: Apache-2.0

//! HMAC-signed cookies for the short-lived upstream OIDC RP flow.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::error::BrokerError;

type HmacSha256 = Hmac<Sha256>;

const SESSION_COOKIE_NAME: &str = "ga4gh_broker_rp_session";

/// Data stored in the RP session cookie during the upstream redirect round-trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpSession {
    /// Configured upstream IdP name.
    pub idp_name: String,
    /// OAuth `state` / CSRF token sent to the upstream IdP.
    pub csrf_state: String,
    /// PKCE code verifier for the authorization code exchange.
    pub pkce_verifier: String,
    /// Optional OIDC nonce when requested from the upstream IdP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    /// Unix timestamp when the session was created.
    pub created_at: i64,
}

/// Signed session cookie helper for RP login state.
pub struct SessionManager {
    key: Vec<u8>,
    lifetime: Duration,
    secure: bool,
}

impl SessionManager {
    /// Create a session manager from a secret string and lifetime.
    pub fn new(secret: &str, lifetime_seconds: u64, secure: bool) -> Self {
        Self {
            key: secret.as_bytes().to_vec(),
            lifetime: Duration::from_secs(lifetime_seconds),
            secure,
        }
    }

    /// Build a `Set-Cookie` header value for an in-progress login session.
    pub fn create_set_cookie(&self, session: &RpSession) -> Result<String, BrokerError> {
        let payload = serde_json::to_string(session)
            .map_err(|err| BrokerError::Internal(format!("serializing session: {err}")))?;
        let encoded = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        let signature = sign_value(&encoded, &self.key)?;
        let secure = if self.secure { "; Secure" } else { "" };
        Ok(format!(
            "{SESSION_COOKIE_NAME}={encoded}.{signature}; HttpOnly{secure}; SameSite=Lax; Path=/; Max-Age={}",
            self.lifetime.as_secs()
        ))
    }

    /// Parse and validate a signed RP session cookie value.
    pub fn parse_cookie_value(&self, raw: &str) -> Result<RpSession, BrokerError> {
        let (encoded, signature) = raw.rsplit_once('.').ok_or(BrokerError::InvalidSession)?;
        verify_value(encoded, signature, &self.key)?;

        let payload = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| BrokerError::InvalidSession)?;
        let session: RpSession =
            serde_json::from_slice(&payload).map_err(|_| BrokerError::InvalidSession)?;
        let now = unix_now();
        if now.saturating_sub(session.created_at) > self.lifetime.as_secs() as i64 {
            return Err(BrokerError::InvalidSession);
        }
        Ok(session)
    }

    /// Build a `Set-Cookie` header that clears the RP session cookie.
    pub fn clear_set_cookie(&self) -> String {
        let secure = if self.secure { "; Secure" } else { "" };
        format!("{SESSION_COOKIE_NAME}=; HttpOnly{secure}; SameSite=Lax; Path=/; Max-Age=0")
    }
}

fn sign_value(value: &str, key: &[u8]) -> Result<String, BrokerError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|err| BrokerError::Internal(format!("session HMAC key: {err}")))?;
    mac.update(value.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn verify_value(value: &str, signature: &str, key: &[u8]) -> Result<(), BrokerError> {
    let expected = sign_value(value, key)?;
    if expected != signature {
        return Err(BrokerError::InvalidSession);
    }
    Ok(())
}

/// Current Unix timestamp in seconds.
pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rp_session_cookie_round_trip() {
        let manager = SessionManager::new("test-secret", 600, false);
        let session = RpSession {
            idp_name: "my-institute".to_string(),
            csrf_state: "state-123".to_string(),
            pkce_verifier: "verifier-456".to_string(),
            nonce: Some("nonce-789".to_string()),
            created_at: unix_now(),
        };

        let header = manager.create_set_cookie(&session).expect("create cookie");
        let raw = header
            .split(';')
            .next()
            .and_then(|part| part.strip_prefix("ga4gh_broker_rp_session="))
            .expect("cookie value");
        let parsed = manager.parse_cookie_value(raw).expect("parse cookie");
        assert_eq!(session, parsed);
    }

    #[test]
    fn rejects_expired_session_cookie() {
        let manager = SessionManager::new("test-secret", 600, false);
        let session = RpSession {
            idp_name: "my-institute".to_string(),
            csrf_state: "state-123".to_string(),
            pkce_verifier: "verifier-456".to_string(),
            nonce: None,
            created_at: unix_now() - 3600,
        };
        let header = manager.create_set_cookie(&session).expect("create cookie");
        let raw = header
            .split(';')
            .next()
            .and_then(|part| part.strip_prefix("ga4gh_broker_rp_session="))
            .expect("cookie value");
        assert!(manager.parse_cookie_value(raw).is_err());
    }

    #[test]
    fn rejects_tampered_session_signature() {
        let manager = SessionManager::new("test-secret", 600, false);
        let session = RpSession {
            idp_name: "my-institute".to_string(),
            csrf_state: "state-123".to_string(),
            pkce_verifier: "verifier-456".to_string(),
            nonce: None,
            created_at: unix_now(),
        };
        let header = manager.create_set_cookie(&session).expect("create cookie");
        let raw = header
            .split(';')
            .next()
            .and_then(|part| part.strip_prefix("ga4gh_broker_rp_session="))
            .expect("cookie value");
        let tampered = format!("{raw}tampered");
        assert!(manager.parse_cookie_value(&tampered).is_err());
    }

    #[test]
    fn pkce_verifier_survives_cookie_round_trip() {
        let manager = SessionManager::new("test-secret", 600, false);
        let verifier = "pkce-verifier-with-special-chars-_~";
        let session = RpSession {
            idp_name: "mock-idp".to_string(),
            csrf_state: "csrf-token".to_string(),
            pkce_verifier: verifier.to_string(),
            nonce: Some("nonce".to_string()),
            created_at: unix_now(),
        };
        let header = manager.create_set_cookie(&session).expect("cookie");
        let raw = header
            .split(';')
            .next()
            .and_then(|part| part.strip_prefix("ga4gh_broker_rp_session="))
            .expect("value");
        let parsed = manager.parse_cookie_value(raw).expect("parse");
        assert_eq!(parsed.pkce_verifier, verifier);
        assert_eq!(parsed.csrf_state, "csrf-token");
    }
}
