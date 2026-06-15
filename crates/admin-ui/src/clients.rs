use std::sync::Arc;
use std::time::Duration;

use ga4gh_types::{AccessRequest, Dataset, DatasetListResponse, DacActionRequest, DacQueueResponse};
use reqwest::Client;
use serde::Deserialize;

use crate::config::AdminUiConfig;
use crate::error::{AdminResult, AdminUiError};

#[derive(Clone)]
pub struct UpstreamClients {
    http: Client,
    config: Arc<AdminUiConfig>,
}

impl UpstreamClients {
    pub fn new(config: Arc<AdminUiConfig>) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client");
        Self { http, config }
    }

    pub async fn ads_dac_queue(&self) -> AdminResult<Vec<AccessRequest>> {
        let url = format!("{}/ads/v1/dac/requests", self.config.ads_base_url.trim_end_matches('/'));
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.config.ads_dac_api_key)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "ADS DAC queue returned {}",
                resp.status()
            )));
        }

        let body: DacQueueResponse = resp
            .json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        Ok(body.requests)
    }

    pub async fn ads_list_datasets(&self) -> AdminResult<Vec<Dataset>> {
        let url = format!("{}/ads/v1/datasets", self.config.ads_base_url.trim_end_matches('/'));
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.config.ads_dac_api_key)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "ADS datasets returned {}",
                resp.status()
            )));
        }

        let body: DatasetListResponse = resp
            .json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        Ok(body.datasets)
    }

    pub async fn ads_get_dataset(&self, id: uuid::Uuid) -> AdminResult<Dataset> {
        let url = format!(
            "{}/ads/v1/datasets/{}",
            self.config.ads_base_url.trim_end_matches('/'),
            id
        );
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.config.ads_dac_api_key)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;

        if resp.status().as_u16() == 404 {
            return Err(AdminUiError::NotFound);
        }
        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "ADS dataset returned {}",
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
    }

    pub async fn ads_approve(&self, id: uuid::Uuid) -> AdminResult<()> {
        self.ads_dac_action(id, "approve").await
    }

    pub async fn ads_reject(&self, id: uuid::Uuid) -> AdminResult<()> {
        self.ads_dac_action(id, "reject").await
    }

    pub async fn ads_escalate(&self, id: uuid::Uuid) -> AdminResult<()> {
        self.ads_dac_action(id, "escalate").await
    }

    async fn ads_dac_action(&self, id: uuid::Uuid, action: &str) -> AdminResult<()> {
        let url = format!(
            "{}/ads/v1/dac/requests/{}/{}",
            self.config.ads_base_url.trim_end_matches('/'),
            id,
            action
        );
        let body = DacActionRequest {
            reason: None,
            actor: Some("admin-ui".to_string()),
        };
        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", &self.config.ads_dac_api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "ADS {action} returned {}",
                resp.status()
            )));
        }
        Ok(())
    }

    pub async fn ads_create_dataset(
        &self,
        payload: &ga4gh_types::CreateDatasetRequest,
    ) -> AdminResult<Dataset> {
        let url = format!("{}/ads/v1/datasets", self.config.ads_base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", &self.config.ads_dac_api_key)
            .json(payload)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AdminUiError::BadRequest(format!("ADS create: {status} {body}")));
        }

        resp.json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
    }

    pub async fn duo_terms(&self) -> AdminResult<Vec<DuoTermOption>> {
        let url = format!("{}/terms", self.config.duo_base_url.trim_end_matches('/'));
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "DUO service returned {}",
                resp.status()
            )));
        }

        #[derive(Deserialize)]
        struct TermsResponse {
            terms: Vec<DuoTermOption>,
        }

        let body: TermsResponse = resp
            .json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        Ok(body.terms)
    }

    pub async fn service_info_ok(&self, base: &str) -> bool {
        let url = format!("{}/service-info", base.trim_end_matches('/'));
        self.http
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

/// DUO term row from duo-service `GET /terms`.
#[derive(Debug, Clone, Deserialize)]
pub struct DuoTermOption {
    pub code: String,
    pub obo_id: String,
    pub label: String,
    pub definition: String,
    pub category: String,
    pub obsolete: bool,
}
