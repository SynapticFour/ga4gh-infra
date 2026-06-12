// SPDX-License-Identifier: Apache-2.0

//! Shared GA4GH data structures for Passport, Visa, DUO, and Service Info.
//!
//! This crate provides serde-compatible types aligned with:
//! - GA4GH Passport & Visa specification v1.2
//! - GA4GH Service Info v1.0.0
//! - Data Use Ontology (DUO) term codes

#![deny(missing_docs)]

pub mod duo;
pub mod passport;
pub mod service_info;
pub mod visa;

pub use duo::{DuoCode, DuoCodeError};
pub use passport::{Passport, PassportClaims};
pub use service_info::{ServiceInfo, ServiceOrganization, ServiceType};
pub use visa::{
    ConditionMatch, ConditionMatchType, Visa, VisaAuthority, VisaClaim, VisaConditionClause,
    VisaConditions, VisaJwtClaims, VisaType, VisaTypeError,
};
