// SPDX-License-Identifier: Apache-2.0

//! Push ADS visa claims to the visa registry for JWT signing.

use ga4gh_types::VisaClaim;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
            .build()
            .map_err(|err| AdsError::Internal(err.to_string()))?;
        Ok(Self {
            base_url: config.url.trim_end_matches('/').to_string(),
            api_key,
            http,
        })
    }

    #[instrument(skip(self, claims))]
    pub async fn publish_and_fetch_jwts(
        &self,
        sub: &str,
        claims: &[VisaClaim],
    ) -> Result<Vec<String>, AdsError> {
        for claim in claims {
            self.create_visa(sub, claim).await?;
        }
        self.list_signed_visas(sub).await
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
