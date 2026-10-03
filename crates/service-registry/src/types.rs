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

/// `GET /services` row. `updatedAt` is the server write time. `stale` is computed
/// at read time and is not stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceView {
    /// Service document with server `updatedAt`.
    #[serde(flatten)]
    pub service: ExternalService,
    /// True when `stale_after_seconds` is set and the row is older than that age.
    pub stale: bool,
}

/// Overlay the SQL timestamp and compute `stale`. A client-supplied `updatedAt` is dropped.
pub fn present_service(
    mut stored: crate::store::StoredService,
    now_unix: i64,
    stale_after_seconds: Option<u64>,
) -> ServiceView {
    stored.service.info.updated_at = Some(format_rfc3339(stored.updated_at));
    let stale = match stale_after_seconds {
        None => false,
        Some(age) => {
            now_unix.saturating_sub(stored.updated_at) > i64::try_from(age).unwrap_or(i64::MAX)
        }
    };
    ServiceView {
        service: stored.service,
        stale,
    }
}

fn format_rfc3339(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix, 0)
        .map(|stamp| stamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use ga4gh_types::{ServiceOrganization, ServiceType};

    use super::*;
    use crate::store::StoredService;

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

    #[test]
    fn unset_age_never_marks_stale_and_drops_client_updated_at() {
        let mut service = ExternalService {
            info: ServiceInfo {
                id: "org.example.wes".to_string(),
                name: "WES".to_string(),
                r#type: ServiceType {
                    group: "org.ga4gh".to_string(),
                    artifact: "wes".to_string(),
                    version: "1.1.0".to_string(),
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
                updated_at: Some("1999-01-01T00:00:00Z".to_string()),
                environment: None,
            },
            url: "https://wes.example.org".to_string(),
        };
        let view = present_service(
            StoredService {
                service: service.clone(),
                updated_at: 1_700_000_000,
            },
            1_700_000_000 + 10_000,
            None,
        );
        assert!(!view.stale);
        assert_eq!(
            view.service.info.updated_at.as_deref(),
            Some("2023-11-14T22:13:20Z")
        );
        service.info.updated_at = None;
        let aged = present_service(
            StoredService {
                service,
                updated_at: 1_000,
            },
            1_000 + 50,
            Some(10),
        );
        assert!(aged.stale);
    }
}
