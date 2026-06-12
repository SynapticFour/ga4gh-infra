// SPDX-License-Identifier: Apache-2.0

//! GA4GH AAI OIDC broker library: upstream RP login and Passport issuance.

pub mod ads;
pub mod app;
pub mod config;
pub mod error;
pub mod handlers;
pub mod identity;
pub mod keys;
pub mod passport;
pub mod profile;
pub mod session;
pub mod startup;
pub mod upstream;
pub mod visas;

#[cfg(test)]
mod test_support;

pub use app::{build_router, AppState};
pub use config::BrokerConfig;
pub use error::BrokerError;
pub use startup::{run, validate_log_level};
