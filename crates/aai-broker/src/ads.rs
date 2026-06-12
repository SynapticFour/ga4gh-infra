// SPDX-License-Identifier: Apache-2.0

//! Access Decision Service (ADS) integration for researcher sync and signed visas.

use std::collections::BTreeMap;

use ga4gh_types::{ResearcherSyncRequest, SignedVisasResponse};
use reqwest::Client;
use serde_json::Value;
use tracing::instrument;

use crate::config::AdsIntegrationConfig;
use crate::error::BrokerError;
use crate::identity::ResearcherIdentity;

/// HTTP client for optional ADS integration.
#[derive(Clone)]
pub struct AdsClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl AdsClient {
    /// Build an ADS client from configuration and environment.
    pub fn new(config: &AdsIntegrationConfig) -> Result<Self, BrokerError> {
        let api_key = std::env::var(&config.sync_api_key_env).map_err(|err| {
            BrokerError::Config(format!(
                "ADS sync API key env `{}`: {err}",
                config.sync_api_key_env
            ))
        })?;
        let http = Client::builder()
            .use_rustls_tls()
            .build()
            .map_err(|err| BrokerError::Internal(format!("ADS HTTP client: {err}")))?;
        Ok(Self {
            base_url: config.url.trim_end_matches('/').to_string(),
            api_key,
            http,
        })
    }

    /// Upsert researcher profile and apply institutional permission mappings.
    #[instrument(skip(self, identity, claims))]
    pub async fn sync_researcher(
        &self,
        identity: &ResearcherIdentity,
        claims: &BTreeMap<String, Value>,
    ) -> Result<(), BrokerError> {
        let request = ResearcherSyncRequest {
            sub: identity.sub.clone(),
            display_name: claims
                .get("name")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            email: identity.email.clone(),
            claims: claims.clone(),
            affiliations: identity
                .affiliation
                .as_ref()
                .map(|organization| {
                    vec![ga4gh_types::ResearcherAffiliation {
                        organization: organization.clone(),
                        role: "member".to_string(),
                    }]
                })
                .unwrap_or_default(),
        };

        let url = format!("{}/ads/v1/researchers/sync", self.base_url);
        let response = self
            .http
            .post(url)
            .header("X-API-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|err| BrokerError::VisaSource(format!("ADS sync failed: {err}")))?;

        if !response.status().is_success() {
            return Err(BrokerError::VisaSource(format!(
                "ADS sync returned HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Fetch signed visa JWTs for passport assembly.
    #[instrument(skip(self), fields(sub = %sub))]
    pub async fn fetch_signed_visas(&self, sub: &str) -> Result<Vec<String>, BrokerError> {
        let url = format!(
            "{}/ads/v1/researchers/{}/signed-visas",
            self.base_url,
            urlencoding::encode(sub)
        );
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|err| BrokerError::VisaSource(format!("ADS signed-visas failed: {err}")))?;

        if !response.status().is_success() {
            return Err(BrokerError::VisaSource(format!(
                "ADS signed-visas returned HTTP {}",
                response.status()
            )));
        }

        let body = response
            .json::<SignedVisasResponse>()
            .await
            .map_err(|err| BrokerError::VisaSource(format!("ADS signed-visas JSON: {err}")))?;

        Ok(body.visa_jwts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn sync_and_fetch_signed_visas() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ads/v1/researchers/sync"))
            .and(header("X-API-Key", "ads-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ads/v1/researchers/researcher-1/signed-visas"))
            .and(header("X-API-Key", "ads-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "researcher_id": "researcher-1",
                "visa_jwts": ["signed-jwt-1"]
            })))
            .mount(&server)
            .await;

        std::env::set_var("TEST_ADS_API_KEY", "ads-key");
        let client = AdsClient::new(&AdsIntegrationConfig {
            url: server.uri(),
            sync_api_key_env: "TEST_ADS_API_KEY".to_string(),
        })
        .expect("client");

        let identity = ResearcherIdentity {
            sub: "researcher-1".to_string(),
            email: Some("r@example.org".to_string()),
            affiliation: None,
            extra: Default::default(),
        };
        client
            .sync_researcher(&identity, &BTreeMap::new())
            .await
            .expect("sync");
        let visas = client
            .fetch_signed_visas("researcher-1")
            .await
            .expect("visas");
        assert_eq!(visas, vec!["signed-jwt-1"]);
    }
}
