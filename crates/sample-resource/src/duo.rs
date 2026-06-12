// SPDX-License-Identifier: Apache-2.0

//! DUO service client for dataset summary endpoints.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::SampleResourceError;

/// Request body sent to the DUO service `/match` endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct DuoMatchRequest {
    /// DUO codes attached to the dataset.
    pub dataset_duo: Vec<String>,
    /// DUO codes describing the researcher's intended use.
    pub intended_use: Vec<String>,
}

/// Response body returned by the DUO service `/match` endpoint.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DuoMatchResponse {
    /// Whether the intended use satisfies the dataset DUO policy.
    pub permitted: bool,
    /// Human-readable explanation of the decision.
    pub reason: String,
}

/// Evaluate dataset DUO restrictions against intended use via the DUO service.
#[instrument(skip(client, duo_service_url, dataset_duo, intended_use))]
pub async fn evaluate_duo_match(
    client: &Client,
    duo_service_url: &str,
    dataset_duo: &[String],
    intended_use: &[String],
) -> Result<DuoMatchResponse, SampleResourceError> {
    let url = format!("{}/match", duo_service_url.trim_end_matches('/'));
    let response = client
        .post(url)
        .json(&DuoMatchRequest {
            dataset_duo: dataset_duo.to_vec(),
            intended_use: intended_use.to_vec(),
        })
        .send()
        .await
        .map_err(|err| SampleResourceError::DuoService(err.to_string()))?;

    if !response.status().is_success() {
        return Err(SampleResourceError::DuoService(format!(
            "unexpected status {}",
            response.status()
        )));
    }

    response
        .json::<DuoMatchResponse>()
        .await
        .map_err(|err| SampleResourceError::DuoService(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn posts_match_request_to_duo_service() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/match"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "permitted": true,
                "reason": "all dataset permissions satisfied",
                "matched_codes": ["GRU"],
                "missing_codes": [],
            })))
            .mount(&server)
            .await;

        let client = Client::new();
        let result = evaluate_duo_match(
            &client,
            &server.uri(),
            &["GRU".to_string()],
            &["HMB".to_string()],
        )
        .await
        .expect("match");

        assert!(result.permitted);
    }
}
