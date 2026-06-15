// SPDX-License-Identifier: Apache-2.0

//! HTTP handlers for the ADS API.

mod access_requests;
mod audit;
mod dac;
mod datasets;
mod duo;
mod grants;
mod introspect;
mod permissions;
mod projects;
mod researchers;
mod service_info;
mod sync;

pub use access_requests::{create_access_request, get_access_request};
pub use audit::list_audit_events;
pub use dac::{dac_approve, dac_escalate, dac_reject, list_dac_requests};
pub use datasets::{create_dataset, get_dataset, list_datasets};
pub use duo::evaluate_duo;
pub use grants::{get_grant, list_grants, revoke_grant};
pub use introspect::introspect;
pub use permissions::{
    create_permission_mapping, create_permission_source, delete_permission_mapping,
    list_permission_mappings, list_permission_sources,
};
pub use projects::{create_project, get_project, list_projects};
pub use researchers::{get_researcher, get_researcher_signed_visas, get_researcher_visas};
pub use service_info::service_info;
pub use sync::sync_researcher_handler;
