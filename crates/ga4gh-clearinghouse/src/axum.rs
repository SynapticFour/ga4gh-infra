// SPDX-License-Identifier: Apache-2.0

//! Axum integration for Passport validation at request boundaries.

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use ga4gh_types::Passport;
use serde::Serialize;

use crate::clearinghouse::Clearinghouse;
use crate::error::ClearinghouseError;

/// Axum application state required by [`ExtractedPassport`].
pub trait ClearinghouseState {
    /// Return the shared clearinghouse instance.
    fn clearinghouse(&self) -> &Arc<Clearinghouse>;
}

impl ClearinghouseState for Arc<Clearinghouse> {
    fn clearinghouse(&self) -> &Arc<Clearinghouse> {
        self
    }
}

impl<T> ClearinghouseState for Arc<T>
where
    T: ClearinghouseState + ?Sized,
{
    fn clearinghouse(&self) -> &Arc<Clearinghouse> {
        (**self).clearinghouse()
    }
}

/// Validated GA4GH Passport extracted from an `Authorization: Bearer` header.
pub struct ExtractedPassport(pub Passport);

/// GA4GH-shaped JSON error body returned by the extractor.
#[derive(Debug, Serialize)]
struct ErrorBody {
    /// Human-readable error message.
    message: String,
}

impl IntoResponse for ClearinghouseError {
    fn into_response(self) -> Response {
        let status = match self {
            ClearinghouseError::ExpiredPassport
            | ClearinghouseError::ExpiredVisa
            | ClearinghouseError::InvalidSignature
            | ClearinghouseError::UntrustedIssuer
            | ClearinghouseError::UnknownKeyId(_) => StatusCode::UNAUTHORIZED,
            ClearinghouseError::InvalidTokenFormat | ClearinghouseError::InvalidClaims(_) => {
                StatusCode::BAD_REQUEST
            }
            ClearinghouseError::JwksFetchFailed(_) | ClearinghouseError::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let message = match self {
            ClearinghouseError::ExpiredPassport => "Expired passport".to_string(),
            ClearinghouseError::ExpiredVisa => "Expired visa".to_string(),
            ClearinghouseError::InvalidSignature => "Invalid passport signature".to_string(),
            ClearinghouseError::UntrustedIssuer => "Untrusted passport issuer".to_string(),
            ClearinghouseError::UnknownKeyId(_) => "Unknown signing key".to_string(),
            ClearinghouseError::InvalidTokenFormat => "Invalid passport token format".to_string(),
            ClearinghouseError::InvalidClaims(_) => "Invalid passport claims".to_string(),
            ClearinghouseError::JwksFetchFailed(_) => "Unable to fetch issuer keys".to_string(),
            ClearinghouseError::Internal(_) => "Internal clearinghouse error".to_string(),
        };

        (status, Json(ErrorBody { message })).into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for ExtractedPassport
where
    S: Send + Sync + ClearinghouseState,
{
    type Rejection = ClearinghouseError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let clearinghouse = state.clearinghouse();
        let token = bearer_token(parts).ok_or(ClearinghouseError::InvalidTokenFormat)?;
        let passport = clearinghouse.validate_passport(token).await?;
        Ok(Self(passport))
    }
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}
