// SPDX-License-Identifier: Apache-2.0

//! HTTP handlers for the sample resource service.

mod datasets;
mod service_info;

pub use datasets::{get_dataset, get_dataset_summary};
pub use service_info::service_info;
