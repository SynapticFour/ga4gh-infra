// SPDX-License-Identifier: Apache-2.0

//! Server-side Visa documents for Passport-scoped access tokens that omit
//! embedded `ga4gh_passport_v1` visas (ADR-005).
//!
//! Entries are keyed by Passport `jti` and expire with the Passport `exp`
//! (TTL bound). The store also enforces a max entry count and a max total
//! visa-byte budget. Multi-replica brokers need a shared store before
//! `embed_visas = false` is safe in HA.
//!
//! # Restart behaviour (fail closed)
//!
//! The store is **in-memory per process**. A broker restart drops every
//! document. A still-valid access token with an empty `ga4gh_passport_v1`
//! then finds no document on `GET /userinfo`; the handler returns **401**
//! (fail closed). Clients must re-authenticate. Do not invent Visas or
//! fall back to embedding after restart.

use std::collections::HashMap;

use tokio::sync::RwLock;

/// Default max Passport documents held for `/userinfo` (ADR-005).
pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

/// Default max sum of Visa JWT string bytes across all documents.
pub const DEFAULT_MAX_TOTAL_VISA_BYTES: usize = 64 * 1024 * 1024;

/// Visas held for one Passport until `exp`.
#[derive(Debug, Clone)]
pub struct StoredVisaDocument {
    /// Passport subject; must match the access-token `sub` on `/userinfo`.
    pub sub: String,
    /// Passport `exp` (unix seconds). Documents with `exp <= now` are treated as gone.
    pub exp: i64,
    /// Visa JWT strings (same bytes that would have been embedded).
    pub visas: Vec<String>,
}

impl StoredVisaDocument {
    fn visa_bytes(&self) -> usize {
        self.visas.iter().map(|v| v.len()).sum()
    }
}

/// Bounds for the in-memory Visa document store (ADR-005).
#[derive(Debug, Clone, Copy)]
pub struct VisaDocumentStoreLimits {
    /// Maximum number of live (non-expired) documents.
    pub max_entries: usize,
    /// Maximum total Visa JWT bytes across all live documents.
    pub max_total_visa_bytes: usize,
}

impl Default for VisaDocumentStoreLimits {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            max_total_visa_bytes: DEFAULT_MAX_TOTAL_VISA_BYTES,
        }
    }
}

/// Why an insert was refused (fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisaDocumentStoreError {
    /// Live entry count would exceed [`VisaDocumentStoreLimits::max_entries`].
    MaxEntries,
    /// Total Visa bytes would exceed [`VisaDocumentStoreLimits::max_total_visa_bytes`].
    MaxBytes,
    /// Document `exp` is not in the future relative to `now`.
    AlreadyExpired,
}

impl std::fmt::Display for VisaDocumentStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxEntries => write!(f, "visa document store at max entries"),
            Self::MaxBytes => write!(f, "visa document store at max total visa bytes"),
            Self::AlreadyExpired => write!(f, "visa document already expired"),
        }
    }
}

impl std::error::Error for VisaDocumentStoreError {}

/// In-memory Visa document cache keyed by Passport `jti`.
pub struct VisaDocumentStore {
    inner: RwLock<HashMap<String, StoredVisaDocument>>,
    limits: VisaDocumentStoreLimits,
}

impl Default for VisaDocumentStore {
    fn default() -> Self {
        Self::new(VisaDocumentStoreLimits::default())
    }
}

impl VisaDocumentStore {
    /// Create a store with explicit size bounds.
    pub fn new(limits: VisaDocumentStoreLimits) -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            limits,
        }
    }

    /// Store visas for a Passport that omitted them from the access token.
    ///
    /// Purges expired entries first. Fails closed when limits would be exceeded
    /// or `document.exp <= now` (TTL bound is the Passport `exp`).
    pub async fn insert(
        &self,
        jti: String,
        document: StoredVisaDocument,
        now: i64,
    ) -> Result<(), VisaDocumentStoreError> {
        if document.exp <= now {
            return Err(VisaDocumentStoreError::AlreadyExpired);
        }
        let new_bytes = document.visa_bytes();
        let mut guard = self.inner.write().await;
        purge_expired(&mut guard, now);
        // Replace of the same jti frees the previous budget first.
        if let Some(prev) = guard.remove(&jti) {
            let _ = prev;
        }
        if guard.len() >= self.limits.max_entries {
            return Err(VisaDocumentStoreError::MaxEntries);
        }
        let used: usize = guard.values().map(StoredVisaDocument::visa_bytes).sum();
        if used.saturating_add(new_bytes) > self.limits.max_total_visa_bytes {
            return Err(VisaDocumentStoreError::MaxBytes);
        }
        guard.insert(jti, document);
        Ok(())
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

fn purge_expired(map: &mut HashMap<String, StoredVisaDocument>, now: i64) {
    map.retain(|_, doc| doc.exp > now);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(sub: &str, exp: i64, visas: &[&str]) -> StoredVisaDocument {
        StoredVisaDocument {
            sub: sub.to_string(),
            exp,
            visas: visas.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn get_requires_matching_sub_and_unexpired() {
        let store = VisaDocumentStore::default();
        store
            .insert("jti-1".to_string(), doc("alice", 2_000, &["visa-a"]), 1_000)
            .await
            .expect("insert");
        assert!(store.get("jti-1", "alice", 1_000).await.is_some());
        assert!(store.get("jti-1", "bob", 1_000).await.is_none());
        assert!(store.get("jti-1", "alice", 2_001).await.is_none());
        store.remove_jtis(&["jti-1".to_string()]).await;
        assert!(store.get("jti-1", "alice", 1_000).await.is_none());
    }

    #[tokio::test]
    async fn insert_fails_closed_at_max_entries() {
        let store = VisaDocumentStore::new(VisaDocumentStoreLimits {
            max_entries: 1,
            max_total_visa_bytes: DEFAULT_MAX_TOTAL_VISA_BYTES,
        });
        store
            .insert("a".to_string(), doc("alice", 2_000, &["v1"]), 1_000)
            .await
            .expect("first");
        let err = store
            .insert("b".to_string(), doc("bob", 2_000, &["v2"]), 1_000)
            .await
            .expect_err("second");
        assert_eq!(err, VisaDocumentStoreError::MaxEntries);
    }

    #[tokio::test]
    async fn insert_fails_closed_at_max_bytes() {
        let store = VisaDocumentStore::new(VisaDocumentStoreLimits {
            max_entries: 10,
            max_total_visa_bytes: 8,
        });
        let err = store
            .insert(
                "a".to_string(),
                doc("alice", 2_000, &["twelve-bytes"]),
                1_000,
            )
            .await
            .expect_err("over budget");
        assert_eq!(err, VisaDocumentStoreError::MaxBytes);
    }

    #[tokio::test]
    async fn insert_purges_expired_before_enforcing_limits() {
        let store = VisaDocumentStore::new(VisaDocumentStoreLimits {
            max_entries: 1,
            max_total_visa_bytes: DEFAULT_MAX_TOTAL_VISA_BYTES,
        });
        store
            .insert("old".to_string(), doc("alice", 1_500, &["v1"]), 1_000)
            .await
            .expect("old");
        store
            .insert("new".to_string(), doc("bob", 3_000, &["v2"]), 2_000)
            .await
            .expect("purge then insert");
        assert!(store.get("old", "alice", 2_000).await.is_none());
        assert!(store.get("new", "bob", 2_000).await.is_some());
    }

    #[tokio::test]
    async fn insert_rejects_already_expired() {
        let store = VisaDocumentStore::default();
        let err = store
            .insert("a".to_string(), doc("alice", 1_000, &["v"]), 1_000)
            .await
            .expect_err("expired");
        assert_eq!(err, VisaDocumentStoreError::AlreadyExpired);
    }
}
