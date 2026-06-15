// SPDX-License-Identifier: Apache-2.0

//! HTTP error mapping for the agreement registry service.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::error::AgreementRegistryError;

/// Errors returned by HTTP handlers.
#[derive(Debug, thiserror::Error)]
pub enum AgreementRegistryHttpError {
    #[error("{0}")]
    Registry(#[from] AgreementRegistryError),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("invalid request: {0}")]
    BadRequest(String),
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

impl AgreementRegistryHttpError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Registry(AgreementRegistryError::NotFound(_)) => StatusCode::NOT_FOUND,
            Self::Registry(AgreementRegistryError::InvalidInput(_)) | Self::BadRequest(_) => {
                StatusCode::BAD_REQUEST
            }
            Self::Registry(AgreementRegistryError::Parse(_)) | Self::Config(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::Registry(err) => err.to_string(),
            Self::BadRequest(message) => message.clone(),
            Self::Config(_) => "An internal error occurred".to_string(),
        }
    }
}

impl IntoResponse for AgreementRegistryHttpError {
    fn into_response(self) -> Response {
        tracing::warn!(error = %self, "request failed");
        let body = ErrorResponse {
            message: self.public_message(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}
