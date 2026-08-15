// SPDX-License-Identifier: Apache-2.0

//! HTTP handlers for broker endpoints.

pub mod callback;
pub mod login;
pub mod oidc;
pub mod revoke;
pub mod service_info;

pub use callback::callback;
pub use login::{login_default, login_named};
pub use oidc::{jwks, openid_configuration, userinfo};
pub use revoke::{list_revoked_passports, revoke_passports};
pub use service_info::service_info;
