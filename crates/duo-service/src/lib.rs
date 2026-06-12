// SPDX-License-Identifier: Apache-2.0

//! Data Use Ontology term resolution and matching service.

pub mod app;
pub mod config;
pub mod error;
pub mod handlers;
pub mod matcher;
pub mod startup;
pub mod terms;

pub use app::{build_router, AppState};
pub use config::DuoServiceConfig;
pub use error::DuoServiceError;
pub use matcher::{MatchRequest, MatchResponse};
pub use startup::{run, validate_log_level};
pub use terms::{DuoCatalog, DuoTerm};
