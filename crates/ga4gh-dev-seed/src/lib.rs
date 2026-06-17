// SPDX-License-Identifier: Apache-2.0

//! Idempotent demo data for local Docker and test stacks.

pub mod config;
pub mod seed;

pub use config::{SeedConfig, SeedProfile};
pub use seed::{seed_dev_stack, SeedSummary};
