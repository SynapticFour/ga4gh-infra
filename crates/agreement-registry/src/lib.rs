// SPDX-License-Identifier: Apache-2.0

//! Agreement registry library — policy profiles, templates, and compatibility checks.
//!
//! **HTTP service endpoints are not implemented yet** (Phase 8 review checkpoint).
//! This crate provides seed templates, in-memory registry helpers, and tests.

pub mod error;
pub mod registry;
pub mod seeds;

pub use error::AgreementRegistryError;
pub use registry::InMemoryRegistry;
pub use seeds::seed_templates;
