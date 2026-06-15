// SPDX-License-Identifier: Apache-2.0

//! Agreement registry library and HTTP service.

pub mod config;
pub mod demo_profiles;
pub mod error;
pub mod handlers;
pub mod http_error;
pub mod registry;
pub mod seeds;
pub mod startup;

pub mod app;

pub use config::AgreementRegistryConfig;
pub use error::AgreementRegistryError;
pub use http_error::AgreementRegistryHttpError;
pub use registry::InMemoryRegistry;
pub use seeds::seed_templates;
pub use startup::{run, validate_log_level};
