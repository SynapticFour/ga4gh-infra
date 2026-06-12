// SPDX-License-Identifier: Apache-2.0

//! GA4GH Access Decision Service (ADS) API.

pub mod app;
pub mod auth;
pub mod config;
pub mod duo;
pub mod error;
pub mod events;
pub mod handlers;
pub mod permissions;
pub mod startup;
pub mod store;
pub mod visa_registry_client;
pub mod visas;

pub use app::{build_router, AppState};
pub use config::AdsConfig;
pub use error::AdsError;
pub use startup::{run, validate_log_level};
