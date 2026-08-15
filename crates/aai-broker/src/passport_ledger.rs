// SPDX-License-Identifier: Apache-2.0

//! Issued-Passport ledger and emergency denylist.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::error::BrokerError;
use crate::session::unix_now;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IssuedPassport {
    jti: String,
    sub: String,
    exp: i64,
    #[serde(default)]
    visa_jtis: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RevokedPassport {
    jti: String,
    exp: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LedgerFile {
    #[serde(default)]
    issued: Vec<IssuedPassport>,
    #[serde(default)]
    revoked: Vec<RevokedPassport>,
}

#[derive(Default)]
struct LedgerState {
    issued: Vec<IssuedPassport>,
    revoked: HashMap<String, i64>,
}

/// In-memory (optionally file-backed) record of minted and revoked Passports.
#[derive(Clone)]
pub struct PassportLedger {
    inner: Arc<Mutex<LedgerState>>,
    path: Option<PathBuf>,
}

impl PassportLedger {
    /// Empty ledger; persist to `path` when set.
    pub fn new(path: Option<PathBuf>) -> Self {
        let state = path
            .as_ref()
            .and_then(|p| load_file(p).ok())
            .unwrap_or_default();
        Self {
            inner: Arc::new(Mutex::new(state)),
            path,
        }
    }

    /// Record a newly minted Passport so it can be revoked by `sub` or visa `jti`.
    pub async fn record_issue(
        &self,
        jti: String,
        sub: String,
        exp: i64,
        visa_jtis: Vec<String>,
    ) -> Result<(), BrokerError> {
        let mut state = self.inner.lock().await;
        prune_locked(&mut state);
        state.issued.push(IssuedPassport {
            jti,
            sub,
            exp,
            visa_jtis,
        });
        self.persist(&state)
    }

    /// Revoke matching live Passports. Returns the number of newly revoked JTIs.
    pub async fn revoke(
        &self,
        sub: Option<&str>,
        jti: Option<&str>,
        visa_jti: Option<&str>,
    ) -> Result<usize, BrokerError> {
        let mut state = self.inner.lock().await;
        prune_locked(&mut state);
        let now = unix_now();
        let mut to_revoke: Vec<(String, i64)> = Vec::new();
        for issued in &state.issued {
            if issued.exp <= now {
                continue;
            }
            let matches_sub = sub.is_some_and(|s| issued.sub == s);
            let matches_jti = jti.is_some_and(|id| issued.jti == id);
            let matches_visa =
                visa_jti.is_some_and(|id| issued.visa_jtis.iter().any(|visa| visa == id));
            if matches_sub || matches_jti || matches_visa {
                to_revoke.push((issued.jti.clone(), issued.exp));
            }
        }
        if let Some(id) = jti {
            to_revoke.push((id.to_string(), now + 3600));
        }
        let mut added = 0usize;
        for (id, exp) in to_revoke {
            if state.revoked.insert(id, exp).is_none() {
                added += 1;
            }
        }
        self.persist(&state)?;
        Ok(added)
    }

    /// JTIs that are still within their original Passport expiry.
    pub async fn revoked_jtis(&self) -> Vec<String> {
        let mut state = self.inner.lock().await;
        prune_locked(&mut state);
        state.revoked.keys().cloned().collect()
    }

    fn persist(&self, state: &LedgerState) -> Result<(), BrokerError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let file = LedgerFile {
            issued: state.issued.clone(),
            revoked: state
                .revoked
                .iter()
                .map(|(jti, exp)| RevokedPassport {
                    jti: jti.clone(),
                    exp: *exp,
                })
                .collect(),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                BrokerError::Internal(format!("passport ledger directory: {err}"))
            })?;
        }
        let json = serde_json::to_string_pretty(&file)
            .map_err(|err| BrokerError::Internal(format!("passport ledger encode: {err}")))?;
        std::fs::write(path, json)
            .map_err(|err| BrokerError::Internal(format!("passport ledger write: {err}")))
    }
}

fn load_file(path: &Path) -> Result<LedgerState, BrokerError> {
    if !path.exists() {
        return Ok(LedgerState::default());
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|err| BrokerError::Internal(format!("passport ledger read: {err}")))?;
    let file: LedgerFile = serde_json::from_str(&raw)
        .map_err(|err| BrokerError::Internal(format!("passport ledger parse: {err}")))?;
    Ok(LedgerState {
        issued: file.issued,
        revoked: file
            .revoked
            .into_iter()
            .map(|row| (row.jti, row.exp))
            .collect(),
    })
}

fn prune_locked(state: &mut LedgerState) {
    let now = unix_now();
    state.issued.retain(|row| row.exp > now);
    state.revoked.retain(|_, exp| *exp > now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn revoke_by_subject_lists_jti() {
        let ledger = PassportLedger::new(None);
        ledger
            .record_issue(
                "jti-1".into(),
                "researcher@example.org".into(),
                unix_now() + 900,
                vec!["visa-1".into()],
            )
            .await
            .unwrap();
        let added = ledger
            .revoke(Some("researcher@example.org"), None, None)
            .await
            .unwrap();
        assert_eq!(added, 1);
        let jtis = ledger.revoked_jtis().await;
        assert_eq!(jtis, vec!["jti-1".to_string()]);
    }

    #[tokio::test]
    async fn revoke_by_visa_jti() {
        let ledger = PassportLedger::new(None);
        ledger
            .record_issue(
                "jti-2".into(),
                "r@example.org".into(),
                unix_now() + 900,
                vec!["visa-abc".into()],
            )
            .await
            .unwrap();
        let added = ledger.revoke(None, None, Some("visa-abc")).await.unwrap();
        assert_eq!(added, 1);
    }
}
