// SPDX-License-Identifier: Apache-2.0

//! Registry error types and GA4GH-shaped HTTP error responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// Errors surfaced by the service registry.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Configuration could not be loaded or validated.
    #[error("configuration error: {0}")]
    Config(String),
    /// The request body or path parameters were invalid.
    #[error("invalid request: {0}")]
    BadRequest(String),
    /// Registration authentication failed.
    #[error("unauthorized")]
    Unauthorized,
    /// Write operations are disabled in read-only mode.
    #[error("registry is read-only")]
    ReadOnly,
    /// The requested service was not found.
    #[error("service not found")]
    NotFound,
    /// Database operation failed.
    #[error("database error: {0}")]
    Database(String),
    /// Internal server error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl RegistryError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::ReadOnly => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Config(_) | Self::Database(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::BadRequest(message) => message.clone(),
            Self::Unauthorized => "Unauthorized".to_string(),
            Self::ReadOnly => "Registry is read-only".to_string(),
            Self::NotFound => "Service not found".to_string(),
            Self::Config(_) | Self::Database(_) | Self::Internal(_) => {
                "An internal error occurred".to_string()
            }
        }
    }
}

/// GA4GH-style JSON error body with a `message` field.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub message: String,
}

impl IntoResponse for RegistryError {
    fn into_response(self) -> Response {
        tracing::warn!(error = %self, "request failed");
        let body = ErrorResponse {
            message: self.public_message(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}
