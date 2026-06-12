// SPDX-License-Identifier: Apache-2.0

//! Agreement registry errors.

use thiserror::Error;

/// Errors from the agreement registry library.
#[derive(Debug, Error)]
pub enum AgreementRegistryError {
    /// Request referenced an unknown profile or template id.
    #[error("not found: {0}")]
    NotFound(String),
    /// Input failed validation.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// JSON or seed data could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),
}
