// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Registry API.

pub mod app;
pub mod auth;
pub mod config;
pub mod error;
pub mod handlers;
pub mod startup;
pub mod store;
pub mod types;

pub use app::{build_router, AppState};
pub use config::RegistryConfig;
pub use error::RegistryError;
pub use startup::{run, validate_log_level};
pub use types::ExternalService;
