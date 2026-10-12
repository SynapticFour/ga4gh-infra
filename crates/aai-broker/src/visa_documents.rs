// SPDX-License-Identifier: Apache-2.0

//! Server-side Visa documents for Passport-scoped access tokens that omit
//! embedded `ga4gh_passport_v1` visas (ADR-005).
//!
//! Entries are keyed by Passport `jti` and expire with the Passport. Multi-replica
//! brokers need a shared store before `embed_visas = false` is safe in HA.

use std::collections::HashMap;

use tokio::sync::RwLock;

/// Visas held for one Passport until `exp`.
#[derive(Debug, Clone)]
pub struct StoredVisaDocument {
    /// Passport subject; must match the access-token `sub` on `/userinfo`.
    pub sub: String,
    /// Passport `exp` (unix seconds).
    pub exp: i64,
    /// Visa JWT strings (same bytes that would have been embedded).
    pub visas: Vec<String>,
}

/// In-memory Visa document cache keyed by Passport `jti`.
#[derive(Default)]
pub struct VisaDocumentStore {
    inner: RwLock<HashMap<String, StoredVisaDocument>>,
}

impl VisaDocumentStore {
    /// Store visas for a Passport that omitted them from the access token.
    pub async fn insert(&self, jti: String, document: StoredVisaDocument) {
        let mut guard = self.inner.write().await;
        guard.insert(jti, document);
    }

    /// Look up a non-expired document whose `sub` matches.
    pub async fn get(&self, jti: &str, sub: &str, now: i64) -> Option<StoredVisaDocument> {
        let guard = self.inner.read().await;
        guard
            .get(jti)
            .cloned()
            .filter(|doc| doc.exp > now && doc.sub == sub)
    }

    /// Drop documents for the given Passport JTIs (e.g. after revoke).
    pub async fn remove_jtis(&self, jtis: &[String]) {
        if jtis.is_empty() {
            return;
        }
        let mut guard = self.inner.write().await;
        for jti in jtis {
            guard.remove(jti);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_requires_matching_sub_and_unexpired() {
        let store = VisaDocumentStore::default();
        store
            .insert(
                "jti-1".to_string(),
                StoredVisaDocument {
                    sub: "alice".to_string(),
                    exp: 2_000,
                    visas: vec!["visa-a".to_string()],
                },
            )
            .await;
        assert!(store.get("jti-1", "alice", 1_000).await.is_some());
        assert!(store.get("jti-1", "bob", 1_000).await.is_none());
        assert!(store.get("jti-1", "alice", 2_001).await.is_none());
        store.remove_jtis(&["jti-1".to_string()]).await;
        assert!(store.get("jti-1", "alice", 1_000).await.is_none());
    }
}
