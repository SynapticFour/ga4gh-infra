// SPDX-License-Identifier: Apache-2.0

//! HTTP handlers for the DUO service API.

mod health;
mod match_handler;
mod service_info;
mod terms;

pub use health::health;
pub use match_handler::match_duo;
pub use service_info::service_info;
pub use terms::{get_term, list_terms};
