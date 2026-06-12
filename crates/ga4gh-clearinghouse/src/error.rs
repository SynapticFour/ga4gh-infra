// SPDX-License-Identifier: Apache-2.0

//! Clearinghouse error types.

use thiserror::Error;

/// Errors returned while validating Passports, Visas, or policies.
#[derive(Debug, Error)]
pub enum ClearinghouseError {
    /// The JWT string is not a well-formed compact JWS.
    #[error("invalid token format")]
    InvalidTokenFormat,
    /// Signature verification failed.
    #[error("invalid signature")]
    InvalidSignature,
    /// The Passport JWT has expired.
    #[error("expired passport")]
    ExpiredPassport,
    /// A visa JWT has expired.
    #[error("expired visa")]
    ExpiredVisa,
    /// The token issuer is not configured as trusted.
    #[error("untrusted issuer")]
    UntrustedIssuer,
    /// JWKS could not be fetched or parsed.
    #[error("JWKS fetch failed: {0}")]
    JwksFetchFailed(String),
    /// The signing key ID was not found even after JWKS refresh.
    #[error("unknown signing key id: {0}")]
    UnknownKeyId(String),
    /// Token claims could not be parsed into GA4GH types.
    #[error("invalid claims: {0}")]
    InvalidClaims(String),
    /// An internal error occurred.
    #[error("internal error: {0}")]
    Internal(String),
}
