// SPDX-License-Identifier: Apache-2.0

//! Cached researcher profile data returned from `/userinfo`.

use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::identity::ResearcherIdentity;

/// Cached profile entry keyed by researcher subject.
#[derive(Debug, Clone)]
pub struct CachedProfile {
    /// Email address from upstream claims.
    pub email: Option<String>,
    /// Affiliation from upstream claims.
    pub affiliation: Option<String>,
    /// Passport expiry used to expire cached profile entries.
    pub exp: i64,
}

/// In-memory profile cache populated after successful upstream login.
#[derive(Default)]
pub struct ProfileStore {
    inner: RwLock<HashMap<String, CachedProfile>>,
}

impl ProfileStore {
    /// Store a researcher profile keyed by subject.
    pub async fn insert(&self, identity: &ResearcherIdentity, exp: i64) {
        let mut guard = self.inner.write().await;
        guard.insert(
            identity.sub.clone(),
            CachedProfile {
                email: identity.email.clone(),
                affiliation: identity.affiliation.clone(),
                exp,
            },
        );
    }

    /// Look up a cached profile by subject if it has not expired.
    pub async fn get(&self, sub: &str, now: i64) -> Option<CachedProfile> {
        let guard = self.inner.read().await;
        guard.get(sub).cloned().filter(|profile| profile.exp > now)
    }
}
