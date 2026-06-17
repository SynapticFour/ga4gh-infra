// SPDX-License-Identifier: Apache-2.0

//! HTTP handlers for the service registry API.

mod health;
mod service_info;
mod services;

pub use health::health;
pub use service_info::service_info;
pub use services::{
    delete_service, get_service, list_service_types, list_services, register_service,
};
