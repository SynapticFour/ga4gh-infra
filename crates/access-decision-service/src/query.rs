// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;

/// Optional DAC group filter repeated as `?dac_group=...` query parameters.
#[derive(Debug, Default, Deserialize)]
pub struct DacGroupQuery {
    #[serde(default)]
    pub dac_group: Vec<String>,
}

impl DacGroupQuery {
    pub fn filter(&self) -> Option<&[String]> {
        if self.dac_group.is_empty() {
            None
        } else {
            Some(self.dac_group.as_slice())
        }
    }
}
