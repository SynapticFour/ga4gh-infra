// SPDX-License-Identifier: Apache-2.0

//! ADS token introspection client for grant-based access checks.

use ga4gh_types::{IntrospectRequest, IntrospectResponse};
use reqwest::Client;
use tracing::instrument;

use crate::config::AdsSection;
use crate::error::SampleResourceError;

/// HTTP client for ADS `/introspect`.
#[derive(Clone)]
pub struct AdsClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl AdsClient {
    /// Build a client from configuration and environment.
    pub fn new(config: &AdsSection, http: Client) -> Result<Self, SampleResourceError> {
        let api_key = std::env::var(&config.api_key_env).map_err(|err| {
            SampleResourceError::Config(format!("ADS API key env `{}`: {err}", config.api_key_env))
        })?;
        Ok(Self {
            base_url: config.url.trim_end_matches('/').to_string(),
            api_key,
            http,
        })
    }

    /// Returns `true` when ADS reports an active grant for the resource.
    #[instrument(skip(self, passport_jwt))]
    pub async fn is_access_active(
        &self,
        passport_jwt: &str,
        resource: &str,
    ) -> Result<bool, SampleResourceError> {
        let response = self
            .http
            .post(format!("{}/ads/v1/introspect", self.base_url))
            .header("X-API-Key", &self.api_key)
            .json(&IntrospectRequest {
                token: passport_jwt.to_string(),
                resource: resource.to_string(),
                action: Some("read".to_string()),
                dataset_id: None,
            })
            .send()
            .await
            .map_err(|err| SampleResourceError::AdsService(err.to_string()))?;

        if !response.status().is_success() {
            return Err(SampleResourceError::AdsService(format!(
                "ADS introspect returned HTTP {}",
                response.status()
            )));
        }

        let body: IntrospectResponse = response.json().await.map_err(|err| {
            SampleResourceError::AdsService(format!("ADS introspect JSON: {err}"))
        })?;

        Ok(body.active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn introspect_returns_active_flag() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ads/v1/introspect"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "active": true,
                "sub": "researcher-1",
                "grant_ids": ["550e8400-e29b-41d4-a716-446655440000"],
                "duo_codes": ["GRU"],
                "exp": 1_700_000_000
            })))
            .mount(&server)
            .await;

        std::env::set_var("TEST_ADS_INTROSPECT_KEY", "ads-key");
        let client = AdsClient::new(
            &AdsSection {
                url: server.uri(),
                api_key_env: "TEST_ADS_INTROSPECT_KEY".to_string(),
            },
            Client::new(),
        )
        .expect("client");

        assert!(client
            .is_access_active("passport-jwt", "dataset-demo")
            .await
            .expect("introspect"));
    }
}
