// SPDX-License-Identifier: Apache-2.0

//! HTTP handlers for the visa registry API.

mod health;
mod jwks;
mod service_info;
mod visas;

pub use health::health;
pub use jwks::jwks;
pub use service_info::service_info;
pub use visas::{create_visa, delete_visa, list_visas};
