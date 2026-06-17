// SPDX-License-Identifier: Apache-2.0

//! HTTP handlers for the agreement registry service.

pub mod compatibility;
pub mod decisions;
pub mod health;
pub mod profiles;
pub mod service_info;
pub mod templates;

pub use compatibility::compatibility_check;
pub use decisions::list_decisions;
pub use health::health;
pub use profiles::{get_profile, list_profiles, register_profile};
pub use service_info::service_info;
pub use templates::{get_template, list_templates};
