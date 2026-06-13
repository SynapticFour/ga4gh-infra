// SPDX-License-Identifier: Apache-2.0

//! Library surface for the combined `ga4gh-infra` CLI.

pub mod africa;
pub mod all_in_one;
pub mod keygen;

pub use africa::{africa_mode_from_env, AfricaProfile};
pub use all_in_one::{prepare_all_in_one_config, run_all_in_one, AllInOneConfig};
pub use keygen::{generate_default_keys, generate_pem, BROKER_KEY_NAME, REGISTRY_KEY_NAME};
