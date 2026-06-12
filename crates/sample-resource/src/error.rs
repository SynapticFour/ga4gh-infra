// SPDX-License-Identifier: Apache-2.0

//! Sample resource service error types and GA4GH-shaped HTTP error responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ga4gh_clearinghouse::ClearinghouseError;
use serde::Serialize;
use thiserror::Error;

/// Errors surfaced by the sample resource service.
#[derive(Debug, Error)]
pub enum SampleResourceError {
    /// Configuration could not be loaded or validated.
    #[error("configuration error: {0}")]
    Config(String),
    /// The requested dataset was not found.
    #[error("unknown dataset")]
    NotFound,
    /// The request headers or parameters were invalid.
    #[error("invalid request: {0}")]
    BadRequest(String),
    /// Passport validation failed.
    #[error("passport validation failed: {0}")]
    Passport(ClearinghouseError),
    /// Access was denied by a clearinghouse policy check.
    #[error("access denied: {0}")]
    Forbidden(String),
    /// DUO policy evaluation failed or denied access.
    #[error("duo policy denied: {0}")]
    DuoDenied(String),
    /// Upstream DUO service request failed.
    #[error("duo service error: {0}")]
    DuoService(String),
    /// Upstream DUO service request failed.
    #[error("ads service error: {0}")]
    AdsService(String),
    /// Internal server error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl SampleResourceError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Passport(err) => passport_status(err),
            Self::Forbidden(_) | Self::DuoDenied(_) => StatusCode::FORBIDDEN,
            Self::DuoService(_) | Self::AdsService(_) => StatusCode::BAD_GATEWAY,
            Self::Config(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::NotFound => "Unknown dataset".to_string(),
            Self::BadRequest(message) => message.clone(),
            Self::Passport(err) => passport_message(err),
            Self::Forbidden(message) | Self::DuoDenied(message) => message.clone(),
            Self::DuoService(_) => "Unable to evaluate DUO policy".to_string(),
            Self::AdsService(_) => "Unable to evaluate access grant".to_string(),
            Self::Config(_) | Self::Internal(_) => "An internal error occurred".to_string(),
        }
    }
}

fn passport_status(error: &ClearinghouseError) -> StatusCode {
    match error {
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
    }
}

fn passport_message(error: &ClearinghouseError) -> String {
    match error {
        ClearinghouseError::ExpiredPassport => "Expired passport".to_string(),
        ClearinghouseError::ExpiredVisa => "Expired visa".to_string(),
        ClearinghouseError::InvalidSignature => "Invalid passport signature".to_string(),
        ClearinghouseError::UntrustedIssuer => "Untrusted passport issuer".to_string(),
        ClearinghouseError::UnknownKeyId(_) => "Unknown signing key".to_string(),
        ClearinghouseError::InvalidTokenFormat => "Invalid passport token format".to_string(),
        ClearinghouseError::InvalidClaims(_) => "Invalid passport claims".to_string(),
        ClearinghouseError::JwksFetchFailed(_) => "Unable to fetch issuer keys".to_string(),
        ClearinghouseError::Internal(_) => "Internal clearinghouse error".to_string(),
    }
}

impl From<ClearinghouseError> for SampleResourceError {
    fn from(error: ClearinghouseError) -> Self {
        Self::Passport(error)
    }
}

/// GA4GH-style JSON error body with a `message` field.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub message: String,
}

impl IntoResponse for SampleResourceError {
    fn into_response(self) -> Response {
        tracing::warn!(error = %self, "request failed");
        let body = ErrorResponse {
            message: self.public_message(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}
