// SPDX-License-Identifier: Apache-2.0

//! One-time authorization codes for confidential return_url clients (query response mode).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::BrokerError;
use crate::session::unix_now;

const CODE_TTL: Duration = Duration::from_secs(120);

/// Stored passport pending a single `/oauth/token` exchange.
#[derive(Debug, Clone)]
pub struct PendingCode {
    /// GA4GH Passport JWT (never logged).
    pub passport_jwt: String,
    /// Optional upstream OIDC id_token for RP-initiated logout.
    pub id_token: Option<String>,
    /// Expected redirect_uri (exact).
    pub redirect_uri: String,
    /// S256 code_challenge from the confidential client (site-api).
    pub code_challenge: String,
    /// Client nonce (opaque; echoed in the token response).
    pub client_nonce: String,
    /// Passport exp (unix).
    pub passport_exp: i64,
    /// Created at.
    pub created: Instant,
}

/// In-memory one-time code store.
#[derive(Debug, Default)]
pub struct AuthCodeStore {
    inner: Mutex<HashMap<String, PendingCode>>,
}

impl AuthCodeStore {
    /// Insert a new one-time code. Returns the opaque code string.
    pub fn issue(&self, pending: PendingCode) -> Result<String, BrokerError> {
        let code = URL_SAFE_NO_PAD.encode(Uuid::new_v4().as_bytes());
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BrokerError::Internal("auth code store lock".into()))?;
        Self::purge_locked(&mut guard);
        if guard.len() >= 10_000 {
            return Err(BrokerError::TooManyRequests);
        }
        guard.insert(code.clone(), pending);
        Ok(code)
    }

    /// Take (remove) a code if present and not expired.
    pub fn take(&self, code: &str) -> Result<PendingCode, BrokerError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BrokerError::Internal("auth code store lock".into()))?;
        Self::purge_locked(&mut guard);
        let pending = guard
            .remove(code)
            .ok_or(BrokerError::AuthenticationFailed)?;
        if pending.created.elapsed() > CODE_TTL {
            return Err(BrokerError::AuthenticationFailed);
        }
        if pending.passport_exp < unix_now() {
            return Err(BrokerError::AuthenticationFailed);
        }
        Ok(pending)
    }

    fn purge_locked(map: &mut HashMap<String, PendingCode>) {
        map.retain(|_, v| v.created.elapsed() <= CODE_TTL);
    }
}

/// Verify an S256 PKCE code_verifier against a stored challenge.
pub fn verify_s256_challenge(verifier: &str, challenge: &str) -> bool {
    let digest = Sha256::digest(verifier.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(digest);
    computed == challenge
}
