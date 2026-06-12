// SPDX-License-Identifier: Apache-2.0

//! GA4GH Passport and Visa validation for resource service boundaries.
//!
//! This library validates Passports and embedded Visas at a clearinghouse,
//! fetches and caches broker JWKS material, and evaluates simple access policies.

#![deny(missing_docs)]

mod clearinghouse;
mod config;
mod error;
mod jwks;
mod policy;
mod token;

#[cfg(feature = "axum")]
pub mod axum;

pub use clearinghouse::Clearinghouse;
pub use config::{ClearinghouseConfig, TrustedBroker};
pub use error::ClearinghouseError;
pub use policy::{PolicyCheck, PolicyResult};
