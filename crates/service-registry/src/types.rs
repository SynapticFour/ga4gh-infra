// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Registry external service types.

use ga4gh_types::ServiceInfo;
use serde::{Deserialize, Serialize};

use crate::error::RegistryError;

/// A GA4GH service entry stored in the registry (`ServiceInfo` plus base URL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalService {
    /// GA4GH service metadata fields.
    #[serde(flatten)]
    pub info: ServiceInfo,
    /// Base URL of the registered service.
    pub url: String,
}

impl ExternalService {
    /// Validate required registration fields.
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.info.id.trim().is_empty() {
            return Err(RegistryError::BadRequest(
                "service id must not be empty".to_string(),
            ));
        }
        if self.info.name.trim().is_empty() {
            return Err(RegistryError::BadRequest(
                "service name must not be empty".to_string(),
            ));
        }
        if self.url.trim().is_empty() {
            return Err(RegistryError::BadRequest(
                "service url must not be empty".to_string(),
            ));
        }
        if self.info.r#type.group.trim().is_empty()
            || self.info.r#type.artifact.trim().is_empty()
            || self.info.r#type.version.trim().is_empty()
        {
            return Err(RegistryError::BadRequest(
                "service type group, artifact, and version are required".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ga4gh_types::{ServiceOrganization, ServiceType};

    use super::*;

    #[test]
    fn round_trips_json_with_flattened_service_info() {
        let service = ExternalService {
            info: ServiceInfo {
                id: "org.example.broker".to_string(),
                name: "Example Broker".to_string(),
                r#type: ServiceType {
                    group: "org.ga4gh".to_string(),
                    artifact: "passport".to_string(),
                    version: "1.2".to_string(),
                },
                organization: ServiceOrganization {
                    name: "Example".to_string(),
                    url: "https://example.org".to_string(),
                    contact_url: None,
                },
                version: "0.1.0".to_string(),
                description: None,
                documentation_url: None,
                created_at: None,
                updated_at: None,
                environment: Some("test".to_string()),
            },
            url: "https://aai.example.org".to_string(),
        };

        let json = serde_json::to_string(&service).expect("serialize");
        let decoded: ExternalService = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(service, decoded);
        assert!(json.contains("\"url\""));
        assert!(!json.contains("contactUrl"));
    }
}
