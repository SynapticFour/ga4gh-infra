// SPDX-License-Identifier: Apache-2.0

//! Cached researcher profile data returned from `/userinfo`.

use std::collections::HashMap;
use std::sync::RwLock;

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
    pub fn insert(&self, identity: &ResearcherIdentity, exp: i64) {
        if let Ok(mut guard) = self.inner.write() {
            guard.insert(
                identity.sub.clone(),
                CachedProfile {
                    email: identity.email.clone(),
                    affiliation: identity.affiliation.clone(),
                    exp,
                },
            );
        }
    }

    /// Look up a cached profile by subject if it has not expired.
    pub fn get(&self, sub: &str, now: i64) -> Option<CachedProfile> {
        self.inner
            .read()
            .ok()
            .and_then(|guard| guard.get(sub).cloned())
            .filter(|profile| profile.exp > now)
    }
}
