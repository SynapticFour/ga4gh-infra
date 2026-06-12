// SPDX-License-Identifier: Apache-2.0

//! ADS error types and GA4GH-shaped HTTP error responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ga4gh_clearinghouse::ClearinghouseError;
use serde::Serialize;
use thiserror::Error;

/// Errors surfaced by ADS.
#[derive(Debug, Error)]
pub enum AdsError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<ClearinghouseError> for AdsError {
    fn from(err: ClearinghouseError) -> Self {
        match err {
            ClearinghouseError::UntrustedIssuer
            | ClearinghouseError::UnknownKeyId(_)
            | ClearinghouseError::InvalidSignature
            | ClearinghouseError::ExpiredPassport
            | ClearinghouseError::ExpiredVisa => Self::Unauthorized,
            ClearinghouseError::InvalidTokenFormat => {
                Self::BadRequest("invalid token format".to_string())
            }
            ClearinghouseError::InvalidClaims(message) => Self::BadRequest(message),
            other => Self::Internal(other.to_string()),
        }
    }
}

impl AdsError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Config(_) | Self::Database(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    pub fn public_message(&self) -> String {
        match self {
            Self::BadRequest(message) => message.clone(),
            Self::Unauthorized => "Unauthorized".to_string(),
            Self::Forbidden => "Forbidden".to_string(),
            Self::NotFound => "Not found".to_string(),
            Self::Conflict(message) => message.clone(),
            Self::Config(_) | Self::Database(_) | Self::Internal(_) => {
                "An internal error occurred".to_string()
            }
        }
    }
}

/// GA4GH-style JSON error body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

impl IntoResponse for AdsError {
    fn into_response(self) -> Response {
        tracing::warn!(error = %self, "request failed");
        let body = ErrorResponse {
            message: self.public_message(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}
