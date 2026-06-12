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
}

impl TrustedBroker {
    /// Create a trusted broker entry.
    pub fn new(issuer: impl Into<String>, jwks_uri: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            jwks_uri: jwks_uri.into(),
        }
    }
}
