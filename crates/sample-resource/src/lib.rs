// SPDX-License-Identifier: Apache-2.0

//! Reference GA4GH resource service using [`ga4gh-clearinghouse`] at the HTTP boundary.

pub mod app;
pub mod config;
pub mod datasets;
pub mod duo;
pub mod error;
pub mod handlers;
pub mod startup;

pub use app::{build_router, AppState};
pub use config::SampleResourceConfig;
pub use error::SampleResourceError;
