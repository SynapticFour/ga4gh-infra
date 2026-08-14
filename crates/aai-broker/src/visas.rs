// SPDX-License-Identifier: Apache-2.0

//! Visa source HTTP client.

use reqwest::Client;
use serde::Deserialize;
use tracing::{instrument, warn};

use crate::config::VisaSourceConfig;
use crate::error::BrokerError;

/// HTTP client for configured visa sources.
#[derive(Clone)]
pub struct VisaSourceClient {
    name: String,
    base_url: String,
    api_key: String,
    required: bool,
    http: Client,
}

impl VisaSourceClient {
    /// Create a visa source client from configuration.
    pub fn new(config: &VisaSourceConfig) -> Result<Self, BrokerError> {
        let api_key = std::env::var(&config.api_key_env).map_err(|err| {
            BrokerError::Config(format!(
                "visa source `{}` API key env `{}`: {err}",
                config.name, config.api_key_env
            ))
        })?;
        let http = Client::builder()
            .use_rustls_tls()
            .build()
            .map_err(|err| BrokerError::Internal(format!("visa HTTP client: {err}")))?;
        Ok(Self {
            name: config.name.clone(),
            base_url: config.url.trim_end_matches('/').to_string(),
            api_key,
            required: config.required,
            http,
        })
    }

    /// Fetch active visa JWT strings for a researcher subject.
    #[instrument(skip(self), fields(source = %self.name, sub = %sub))]
    pub async fn fetch_visas(&self, sub: &str) -> Result<Vec<String>, BrokerError> {
        let url = format!("{}/visas?sub={}", self.base_url, urlencoding::encode(sub));
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|err| BrokerError::VisaSource(err.to_string()))?;

        if !response.status().is_success() {
            return Err(BrokerError::VisaSource(format!(
                "visa source `{}` returned HTTP {}",
                self.name,
                response.status()
            )));
        }

        let body = response
            .json::<VisaListResponse>()
            .await
            .map_err(|err| BrokerError::VisaSource(err.to_string()))?;

        Ok(body.into_jwts())
    }
}

/// Visa list response shapes accepted from visa sources.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum VisaListResponse {
    /// Plain array of visa JWT strings.
    JwtList(Vec<String>),
    /// Object wrapper with a `visas` array.
    Wrapped { visas: Vec<VisaRecord> },
}

#[derive(Debug, Deserialize)]
struct VisaRecord {
    #[serde(default)]
    jwt: Option<String>,
    #[serde(default)]
    token: Option<String>,
}

impl VisaListResponse {
    fn into_jwts(self) -> Vec<String> {
        match self {
            Self::JwtList(list) => list,
            Self::Wrapped { visas } => visas
                .into_iter()
                .filter_map(|record| record.jwt.or(record.token))
                .collect(),
        }
    }
}

/// Query all configured visa sources and merge visa JWT strings.
#[instrument(skip(sources), fields(sub = %sub))]
pub async fn collect_visas(
    sources: &[VisaSourceClient],
    sub: &str,
) -> Result<Vec<String>, BrokerError> {
    let mut set = tokio::task::JoinSet::new();
    for source in sources {
        let source = source.clone();
        let sub = sub.to_string();
        set.spawn(async move {
            let required = source.required;
            let name = source.name.clone();
            (required, name, source.fetch_visas(&sub).await)
        });
    }

    let mut visas = Vec::new();
    while let Some(joined) = set.join_next().await {
        let (required, name, result) =
            joined.map_err(|err| BrokerError::Internal(format!("visa source task: {err}")))?;
        match result {
            Ok(mut fetched) => visas.append(&mut fetched),
            Err(err) if required => return Err(err),
            Err(err) => warn!(source = %name, error = %err, "optional visa source failed"),
        }
    }
    Ok(visas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fetches_wrapped_visa_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/visas"))
            .and(query_param("sub", "researcher-1"))
            .and(header("X-API-Key", "visa-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "visas": [
                    { "jwt": "visa-jwt-1" },
                    { "token": "visa-jwt-2" }
                ]
            })))
            .mount(&server)
            .await;

        std::env::set_var("TEST_VISA_API_KEY", "visa-key");
        let client = VisaSourceClient::new(&VisaSourceConfig {
            name: "test".to_string(),
            url: server.uri(),
            api_key_env: "TEST_VISA_API_KEY".to_string(),
            required: true,
        })
        .expect("client");

        let visas = client.fetch_visas("researcher-1").await.expect("fetch");
        assert_eq!(visas, vec!["visa-jwt-1", "visa-jwt-2"]);
    }
}
