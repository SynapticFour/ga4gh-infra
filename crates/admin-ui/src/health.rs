//! Service health probing for the dashboard.

use ga4gh_types::ServiceInfo;
use reqwest::Client;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Up,
    Degraded,
    Down,
}

impl HealthStatus {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Up => "health-up",
            Self::Degraded => "health-degraded",
            Self::Down => "health-down",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Degraded => "Degraded",
            Self::Down => "Down",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub name: String,
    pub status: HealthStatus,
    pub version: String,
    pub detail: Option<String>,
}

pub async fn probe_service(http: &Client, name: &str, base_url: &str) -> ServiceHealth {
    let base = base_url.trim_end_matches('/');
    let info_url = format!("{base}/service-info");
    let info_resp = http.get(&info_url).send().await;

    let info = match info_resp {
        Ok(resp) if resp.status().is_success() => resp.json::<ServiceInfo>().await.ok(),
        _ => {
            return ServiceHealth {
                name: name.to_string(),
                status: HealthStatus::Down,
                version: "—".into(),
                detail: Some("service-info unavailable".into()),
            };
        }
    };

    let health_url = format!("{base}/health");
    let health_status = http.get(&health_url).send().await.ok().map(|r| r.status());

    let status = match health_status {
        Some(s) if s.is_success() => HealthStatus::Up,
        Some(s) if s.is_server_error() => HealthStatus::Degraded,
        Some(_) => HealthStatus::Degraded,
        None => HealthStatus::Up,
    };

    let version = info
        .as_ref()
        .map(|i| i.version.clone())
        .unwrap_or_else(|| "—".into());

    ServiceHealth {
        name: name.to_string(),
        status,
        version,
        detail: info.map(|i| format!("{} / {}", i.r#type.artifact, i.r#type.version)),
    }
}

/// Parse broker JWKS for dashboard signing-key summary (public material only).
pub fn signing_key_summary(jwks: &Value) -> Option<SigningKeySummary> {
    let keys = jwks.get("keys")?.as_array()?;
    let key = keys.first()?;
    Some(SigningKeySummary {
        kid: key
            .get("kid")
            .and_then(|v| v.as_str())
            .unwrap_or("—")
            .to_string(),
        algorithm: key
            .get("alg")
            .and_then(|v| v.as_str())
            .unwrap_or("RS256")
            .to_string(),
        key_count: keys.len(),
    })
}

#[derive(Debug, Clone)]
pub struct SigningKeySummary {
    pub kid: String,
    pub algorithm: String,
    pub key_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_key_summary_reads_first_jwk() {
        let jwks = serde_json::json!({
            "keys": [{"kid": "broker-1", "alg": "RS256", "kty": "RSA"}]
        });
        let summary = signing_key_summary(&jwks).expect("summary");
        assert_eq!(summary.kid, "broker-1");
        assert_eq!(summary.algorithm, "RS256");
        assert_eq!(summary.key_count, 1);
    }
}
