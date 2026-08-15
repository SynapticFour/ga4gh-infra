// SPDX-License-Identifier: Apache-2.0

//! Clearinghouse configuration types.

use std::time::Duration;

/// Configuration for a [`crate::Clearinghouse`] instance.
#[derive(Debug, Clone)]
pub struct ClearinghouseConfig {
    /// Trusted Passport brokers and Visa issuers.
    pub trusted_brokers: Vec<TrustedBroker>,
    /// TTL for cached JWKS documents fetched from brokers.
    pub jwks_cache_ttl: Duration,
}

impl ClearinghouseConfig {
    /// Create a configuration with the given trusted brokers and JWKS cache TTL.
    pub fn new(trusted_brokers: Vec<TrustedBroker>, jwks_cache_ttl: Duration) -> Self {
        Self {
            trusted_brokers,
            jwks_cache_ttl,
        }
    }
}

/// A trusted broker or visa issuer and its JWKS endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedBroker {
    /// Expected JWT `iss` claim value.
    pub issuer: String,
    /// JWKS URL used to resolve signing keys for this issuer.
    pub jwks_uri: String,
    /// Optional visa revocation list URL. Defaults to `{jwks origin}/revoked-jtis`.
    pub revocation_uri: Option<String>,
    /// Optional Passport JWT revocation list. Defaults to `{jwks origin}/revoked-passports`.
    pub passport_revocation_uri: Option<String>,
}

impl TrustedBroker {
    /// Create a trusted broker entry. Revocation URLs are inferred from `jwks_uri` when possible.
    pub fn new(issuer: impl Into<String>, jwks_uri: impl Into<String>) -> Self {
        let jwks_uri = jwks_uri.into();
        let base = jwks_uri.strip_suffix("/jwks.json");
        let revocation_uri = base.map(|origin| format!("{origin}/revoked-jtis"));
        let passport_revocation_uri = base.map(|origin| format!("{origin}/revoked-passports"));
        Self {
            issuer: issuer.into(),
            jwks_uri,
            revocation_uri,
            passport_revocation_uri,
        }
    }

    /// Override the inferred visa revocation endpoint.
    pub fn with_revocation_uri(mut self, uri: impl Into<String>) -> Self {
        self.revocation_uri = Some(uri.into());
        self
    }

    /// Override the inferred Passport revocation endpoint.
    pub fn with_passport_revocation_uri(mut self, uri: impl Into<String>) -> Self {
        self.passport_revocation_uri = Some(uri.into());
        self
    }
}
