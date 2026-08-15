// SPDX-License-Identifier: Apache-2.0

//! Broker error types and GA4GH-shaped HTTP error responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// Errors surfaced by the broker.
#[derive(Debug, Error)]
pub enum BrokerError {
    /// Configuration could not be loaded or validated.
    #[error("configuration error: {0}")]
    Config(String),
    /// The requested upstream IdP is not configured.
    #[error("unknown upstream identity provider")]
    UnknownIdp,
    /// The OIDC login or callback flow failed validation.
    #[error("authentication failed")]
    AuthenticationFailed,
    /// Upstream OIDC discovery or token exchange failed.
    #[error("upstream OIDC error: {0}")]
    UpstreamOidc(String),
    /// Visa source query failed.
    #[error("visa source error: {0}")]
    VisaSource(String),
    /// Passport or access-token signing failed.
    #[error("token signing error: {0}")]
    Signing(String),
    /// Bearer token validation failed.
    #[error("invalid access token")]
    InvalidAccessToken,
    /// Session cookie could not be parsed or has expired.
    #[error("invalid session")]
    InvalidSession,
    /// Login or callback rate limit exceeded.
    #[error("too many requests")]
    TooManyRequests,
    /// Missing or invalid admin API key.
    #[error("unauthorized")]
    Unauthorized,
    /// Caller sent a malformed request.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// Internal server error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl BrokerError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownIdp => StatusCode::NOT_FOUND,
            Self::AuthenticationFailed
            | Self::InvalidAccessToken
            | Self::InvalidSession
            | Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Config(_)
            | Self::UpstreamOidc(_)
            | Self::VisaSource(_)
            | Self::Signing(_)
            | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn public_message(&self) -> &'static str {
        match self {
            Self::UnknownIdp => "Unknown upstream identity provider",
            Self::AuthenticationFailed => "Authentication failed",
            Self::TooManyRequests => "Too many requests",
            Self::Unauthorized => "Unauthorized",
            Self::BadRequest(_) => "Invalid request",
            Self::InvalidAccessToken => "Invalid or expired access token",
            Self::InvalidSession => "Invalid or expired login session",
            Self::Config(_)
            | Self::UpstreamOidc(_)
            | Self::VisaSource(_)
            | Self::Signing(_)
            | Self::Internal(_) => "An internal error occurred",
        }
    }
}

/// GA4GH-style JSON error body with a `message` field.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub message: String,
}

impl IntoResponse for BrokerError {
    fn into_response(self) -> Response {
        tracing::warn!(error = %self, "request failed");
        let body = ErrorResponse {
            message: self.public_message().to_string(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}
