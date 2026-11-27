// SPDX-License-Identifier: Apache-2.0

//! Push ADS visa claims to the visa registry for JWT signing.

use ga4gh_types::VisaClaim;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;

use crate::config::VisaRegistryConfig;
use crate::error::AdsError;

#[derive(Clone)]
pub struct VisaRegistryClient {
    base_url: String,
    api_key: String,
    http: Client,
}

#[derive(Debug, Serialize)]
struct CreateVisaBody<'a> {
    sub: &'a str,
    r#type: &'a str,
    value: &'a str,
    source: &'a str,
    asserted: i64,
}

#[derive(Debug, Deserialize)]
struct SignedVisaRecord {
    jwt: Option<String>,
    token: Option<String>,
}

/// The parts of a registry visa JWT that reconciliation compares. The registry
/// is reached with its API key, so its JWTs are read without checking the
/// signature here; Passport consumers still verify them.
#[derive(Debug, Clone, PartialEq)]
struct RegistryVisa {
    jti: String,
    visa_type: String,
    value: String,
    source: String,
}

impl RegistryVisa {
    fn from_jwt(jwt: &str) -> Option<Self> {
        use base64::Engine as _;
        let payload = jwt.split('.').nth(1)?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload.trim_end_matches('='))
            .ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let visa = value.get("ga4gh_visa_v1")?;
        Some(Self {
            jti: value.get("jti")?.as_str()?.to_string(),
            visa_type: visa.get("type")?.as_str()?.to_string(),
            value: visa.get("value")?.as_str()?.to_string(),
            source: visa.get("source")?.as_str()?.to_string(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ListVisasBody {
    Wrapped { visas: Vec<SignedVisaRecord> },
    JwtList(Vec<String>),
}

impl VisaRegistryClient {
    pub fn new(config: &VisaRegistryConfig, api_key: String) -> Result<Self, AdsError> {
        let http = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|err| AdsError::Internal(err.to_string()))?;
        Ok(Self {
            base_url: config.url.trim_end_matches('/').to_string(),
            api_key,
            http,
        })
    }

    /// Make the registry hold exactly `claims` for `sub` among the visas this ADS
    /// manages, then return the signed JWTs.
    ///
    /// A visa counts as managed when its `source` is `managed_source` (the ADS
    /// `default_source_url`). Managed `ControlledAccessGrants` visas that no
    /// active grant backs are deleted (registry revocation, so their `jti` joins
    /// `/revoked-jtis`). Claims already present are not created again, so repeated
    /// calls no longer pile up duplicates. Visas from other sources are left alone.
    #[instrument(skip(self, claims))]
    pub async fn publish_and_fetch_jwts(
        &self,
        sub: &str,
        claims: &[VisaClaim],
        managed_source: &str,
    ) -> Result<Vec<String>, AdsError> {
        let existing: Vec<RegistryVisa> = self
            .list_signed_visas(sub)
            .await?
            .iter()
            .filter_map(|jwt| RegistryVisa::from_jwt(jwt))
            .collect();

        for visa in &existing {
            if visa.source != managed_source || visa.visa_type != "ControlledAccessGrants" {
                continue;
            }
            let still_granted = claims
                .iter()
                .any(|claim| claim.r#type.as_str() == visa.visa_type && claim.value == visa.value);
            if !still_granted {
                self.delete_visa(&visa.jti).await?;
            }
        }

        for claim in claims {
            let present = existing.iter().any(|visa| {
                visa.visa_type == claim.r#type.as_str()
                    && visa.value == claim.value
                    && visa.source == claim.source
            });
            if !present {
                self.create_visa(sub, claim).await?;
            }
        }
        self.list_signed_visas(sub).await
    }

    async fn delete_visa(&self, jti: &str) -> Result<(), AdsError> {
        let response = self
            .http
            .delete(format!("{}/visas/{}", self.base_url, jti))
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|err| AdsError::Internal(format!("visa-registry DELETE: {err}")))?;
        // 404: already gone, which is the state we want.
        if !response.status().is_success() && response.status().as_u16() != 404 {
            return Err(AdsError::Internal(format!(
                "visa-registry DELETE /visas/{{id}} returned HTTP {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn create_visa(&self, sub: &str, claim: &VisaClaim) -> Result<(), AdsError> {
        let body = CreateVisaBody {
            sub,
            r#type: claim.r#type.as_str(),
            value: &claim.value,
            source: &claim.source,
            asserted: claim.asserted,
        };
        let response = self
            .http
            .post(format!("{}/visas", self.base_url))
            .header("X-API-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| AdsError::Internal(format!("visa-registry POST: {err}")))?;

        if !response.status().is_success() {
            return Err(AdsError::Internal(format!(
                "visa-registry POST /visas returned HTTP {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn list_signed_visas(&self, sub: &str) -> Result<Vec<String>, AdsError> {
        let response = self
            .http
            .get(format!("{}/visas", self.base_url))
            .header("X-API-Key", &self.api_key)
            .query(&[("sub", sub)])
            .send()
            .await
            .map_err(|err| AdsError::Internal(format!("visa-registry GET: {err}")))?;

        if !response.status().is_success() {
            return Err(AdsError::Internal(format!(
                "visa-registry GET /visas returned HTTP {}",
                response.status()
            )));
        }

        let body: ListVisasBody = response
            .json()
            .await
            .map_err(|err| AdsError::Internal(format!("visa-registry JSON: {err}")))?;

        Ok(match body {
            ListVisasBody::JwtList(list) => list,
            ListVisasBody::Wrapped { visas } => visas
                .into_iter()
                .filter_map(|record| record.jwt.or(record.token))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use ga4gh_types::VisaType;
    use wiremock::matchers::{body_partial_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const ADS: &str = "https://ads.test";

    fn jwt(jti: &str, visa_type: &str, value: &str, source: &str) -> String {
        let enc = |v: serde_json::Value| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v.to_string())
        };
        format!(
            "{}.{}.sig",
            enc(serde_json::json!({"alg": "RS256", "typ": "JWT"})),
            enc(serde_json::json!({
                "jti": jti,
                "sub": "researcher-1",
                "ga4gh_visa_v1": {"type": visa_type, "value": value, "source": source, "asserted": 1}
            }))
        )
    }

    fn cag(value: &str) -> VisaClaim {
        VisaClaim {
            r#type: VisaType::ControlledAccessGrants,
            asserted: 1,
            value: value.to_string(),
            source: ADS.to_string(),
            by: None,
            conditions: None,
        }
    }

    fn client(server: &MockServer) -> VisaRegistryClient {
        let config = VisaRegistryConfig {
            url: server.uri(),
            api_key_env: "UNUSED".to_string(),
        };
        VisaRegistryClient::new(&config, "key".to_string()).unwrap()
    }

    async fn list_returns(server: &MockServer, jwts: Vec<String>) {
        Mock::given(method("GET"))
            .and(path("/visas"))
            .and(query_param("sub", "researcher-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!(jwts)))
            .mount(server)
            .await;
    }

    #[test]
    fn reads_visa_fields_from_jwt_payload() {
        let visa =
            RegistryVisa::from_jwt(&jwt("j1", "ControlledAccessGrants", "ds-1", ADS)).unwrap();
        assert_eq!(visa.jti, "j1");
        assert_eq!(visa.value, "ds-1");
        assert!(RegistryVisa::from_jwt("not-a-jwt").is_none());
    }

    #[tokio::test]
    async fn revoked_grant_visa_is_deleted() {
        let server = MockServer::start().await;
        list_returns(
            &server,
            vec![jwt("j-old", "ControlledAccessGrants", "ds-1", ADS)],
        )
        .await;
        Mock::given(method("DELETE"))
            .and(path("/visas/j-old"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&server)
            .await;

        client(&server)
            .publish_and_fetch_jwts("researcher-1", &[], ADS)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn present_visa_is_not_created_again() {
        let server = MockServer::start().await;
        list_returns(
            &server,
            vec![jwt("j1", "ControlledAccessGrants", "ds-1", ADS)],
        )
        .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(204))
            .expect(0)
            .mount(&server)
            .await;

        client(&server)
            .publish_and_fetch_jwts("researcher-1", &[cag("ds-1")], ADS)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn missing_visa_is_created_and_foreign_visas_are_kept() {
        let server = MockServer::start().await;
        list_returns(
            &server,
            vec![
                jwt(
                    "j-foreign",
                    "ControlledAccessGrants",
                    "ds-9",
                    "https://other.example",
                ),
                jwt("j-role", "AffiliationAndRole", "ferrum:analyst", ADS),
            ],
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/visas"))
            .and(body_partial_json(serde_json::json!({"value": "ds-2"})))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(204))
            .expect(0)
            .mount(&server)
            .await;

        client(&server)
            .publish_and_fetch_jwts("researcher-1", &[cag("ds-2")], ADS)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn failed_delete_is_an_error() {
        let server = MockServer::start().await;
        list_returns(
            &server,
            vec![jwt("j-old", "ControlledAccessGrants", "ds-1", ADS)],
        )
        .await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        assert!(client(&server)
            .publish_and_fetch_jwts("researcher-1", &[], ADS)
            .await
            .is_err());
    }
}
