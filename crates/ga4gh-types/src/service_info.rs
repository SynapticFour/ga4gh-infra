// SPDX-License-Identifier: Apache-2.0

//! GA4GH Service Info types (v1.0.0).

use serde::{Deserialize, Serialize};

/// GA4GH service type descriptor (`group`, `artifact`, `version`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceType {
    /// Namespace in reverse domain name format (e.g. `org.ga4gh`).
    pub group: String,
    /// Name of the API or GA4GH specification implemented.
    pub artifact: String,
    /// Version of the API or specification.
    pub version: String,
}

/// Organization providing a GA4GH service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceOrganization {
    /// Name of the organization responsible for the service.
    pub name: String,
    /// URL of the organization's website.
    pub url: String,
    /// Contact URL or mailto link for the service provider.
    #[serde(rename = "contactUrl", skip_serializing_if = "Option::is_none")]
    pub contact_url: Option<String>,
}

/// GA4GH `/service-info` response object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Unique service identifier (reverse domain name notation recommended).
    pub id: String,
    /// Human-readable service name.
    pub name: String,
    /// Service type descriptor.
    pub r#type: ServiceType,
    /// Organization providing the service.
    pub organization: ServiceOrganization,
    /// Service version string.
    pub version: String,
    /// Human-readable service description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// URL of service documentation.
    #[serde(rename = "documentationUrl", skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    /// Timestamp when the service was first deployed (RFC 3339).
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Timestamp when the service was last updated (RFC 3339).
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Deployment environment (e.g. `prod`, `test`, `dev`, `staging`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_type_round_trip() {
        let service_type = ServiceType {
            group: "org.ga4gh".to_string(),
            artifact: "passport".to_string(),
            version: "1.2".to_string(),
        };

        let json = serde_json::to_string(&service_type).expect("serialize");
        let decoded: ServiceType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(service_type, decoded);
    }

    #[test]
    fn service_organization_round_trip() {
        let org = ServiceOrganization {
            name: "Example Institute".to_string(),
            url: "https://example.org".to_string(),
            contact_url: Some("mailto:support@example.org".to_string()),
        };

        let json = serde_json::to_string(&org).expect("serialize");
        let decoded: ServiceOrganization = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(org, decoded);
    }

    #[test]
    fn service_info_round_trip() {
        let info = ServiceInfo {
            id: "org.example.ga4gh-broker".to_string(),
            name: "Example GA4GH Broker".to_string(),
            r#type: ServiceType {
                group: "org.ga4gh".to_string(),
                artifact: "passport".to_string(),
                version: "1.2".to_string(),
            },
            organization: ServiceOrganization {
                name: "Example Institute".to_string(),
                url: "https://example.org".to_string(),
                contact_url: None,
            },
            version: "0.1.0".to_string(),
            description: Some("OIDC broker issuing GA4GH Passports".to_string()),
            documentation_url: Some("https://docs.example.org/broker".to_string()),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            updated_at: Some("2024-06-01T00:00:00Z".to_string()),
            environment: Some("test".to_string()),
        };

        let json = serde_json::to_string(&info).expect("serialize");
        let decoded: ServiceInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(info, decoded);
    }
}
