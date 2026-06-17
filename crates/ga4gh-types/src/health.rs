// SPDX-License-Identifier: Apache-2.0

//! Lightweight `/health` response for GA4GH service liveness probes.

use serde::{Deserialize, Serialize};

/// JSON body returned by service `/health` endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// `"ok"` when the process is running and ready to serve traffic.
    pub status: String,
}

impl HealthResponse {
    /// Healthy liveness response.
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_ok_status() {
        let json = serde_json::to_string(&HealthResponse::ok()).expect("serialize");
        assert_eq!(json, r#"{"status":"ok"}"#);
    }
}
