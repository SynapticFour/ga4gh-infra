use std::sync::Arc;
use std::time::Duration;

use ga4gh_types::{
    AccessRequest, AdsEvent, AgreementTemplate, AgreementTemplateListResponse,
    AuditEventListResponse, CompatibilityCheckRequest, CompatibilityCheckResult,
    CreateDatasetRequest, CreatePermissionMappingRequest, CreatePermissionSourceRequest,
    CreateProjectRequest, DacActionRequest, DacQueueResponse, Dataset, DatasetListResponse, Grant,
    GrantListResponse, PermissionMapping, PermissionMappingListResponse, PermissionSource,
    PermissionSourceListResponse, ProjectListResponse, ResearchProject, Researcher,
    SignedVisasResponse,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

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

    fn ads_url(&self, path: &str) -> String {
        format!(
            "{}/ads/v1{}",
            self.config.ads_base_url.trim_end_matches('/'),
            path
        )
    }

    async fn ads_get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        dac_groups: Option<&[String]>,
    ) -> AdminResult<T> {
        let mut req = self
            .http
            .get(self.ads_url(path))
            .header("X-API-Key", &self.config.ads_dac_api_key);
        if let Some(groups) = dac_groups {
            for group in groups {
                req = req.query(&[("dac_group", group.as_str())]);
            }
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        if resp.status().as_u16() == 404 {
            return Err(AdminUiError::NotFound);
        }
        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "ADS GET {path} returned {}",
                resp.status()
            )));
        }
        resp.json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
    }

    async fn ads_post<T: serde::Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> AdminResult<R> {
        let resp = self
            .http
            .post(self.ads_url(path))
            .header("X-API-Key", &self.config.ads_dac_api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AdminUiError::BadRequest(format!(
                "ADS POST {path}: {status} {text}"
            )));
        }
        resp.json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
    }

    async fn ads_delete(&self, path: &str) -> AdminResult<()> {
        let resp = self
            .http
            .delete(self.ads_url(path))
            .header("X-API-Key", &self.config.ads_dac_api_key)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "ADS DELETE {path} returned {}",
                resp.status()
            )));
        }
        Ok(())
    }

    pub async fn ads_dac_queue(
        &self,
        dac_groups: Option<&[String]>,
    ) -> AdminResult<Vec<AccessRequest>> {
        let body: DacQueueResponse = self.ads_get("/dac/requests", dac_groups).await?;
        Ok(body.requests)
    }

    pub async fn ads_list_datasets(
        &self,
        dac_groups: Option<&[String]>,
    ) -> AdminResult<Vec<Dataset>> {
        let body: DatasetListResponse = self.ads_get("/datasets", dac_groups).await?;
        Ok(body.datasets)
    }

    pub async fn ads_get_dataset(&self, id: uuid::Uuid) -> AdminResult<Dataset> {
        self.ads_get(&format!("/datasets/{id}"), None).await
    }

    pub async fn ads_list_projects(&self) -> AdminResult<Vec<ResearchProject>> {
        let body: ProjectListResponse = self.ads_get("/projects", None).await?;
        Ok(body.projects)
    }

    pub async fn ads_get_project(&self, id: uuid::Uuid) -> AdminResult<ResearchProject> {
        self.ads_get(&format!("/projects/{id}"), None).await
    }

    pub async fn ads_list_grants(&self, dac_groups: Option<&[String]>) -> AdminResult<Vec<Grant>> {
        let body: GrantListResponse = self.ads_get("/grants", dac_groups).await?;
        Ok(body.grants)
    }

    pub async fn ads_list_audit(
        &self,
        limit: u32,
        dac_groups: Option<&[String]>,
    ) -> AdminResult<Vec<AdsEvent>> {
        let body: AuditEventListResponse = self
            .ads_get(&format!("/audit/events?limit={limit}"), dac_groups)
            .await?;
        Ok(body.events)
    }

    pub async fn ads_get_researcher(&self, id: &str) -> AdminResult<Researcher> {
        self.ads_get(&format!("/researchers/{id}"), None).await
    }

    pub async fn ads_get_researcher_visas(&self, id: &str) -> AdminResult<SignedVisasResponse> {
        self.ads_get(&format!("/researchers/{id}/signed-visas"), None)
            .await
    }

    pub async fn ads_list_permission_sources(&self) -> AdminResult<Vec<PermissionSource>> {
        let body: PermissionSourceListResponse = self.ads_get("/permission-sources", None).await?;
        Ok(body.sources)
    }

    pub async fn ads_list_permission_mappings(&self) -> AdminResult<Vec<PermissionMapping>> {
        let body: PermissionMappingListResponse =
            self.ads_get("/permission-mappings", None).await?;
        Ok(body.mappings)
    }

    pub async fn ads_approve(&self, id: uuid::Uuid, reason: Option<String>) -> AdminResult<()> {
        self.ads_dac_action(id, "approve", reason).await
    }

    pub async fn ads_reject(&self, id: uuid::Uuid, reason: Option<String>) -> AdminResult<()> {
        self.ads_dac_action(id, "reject", reason).await
    }

    pub async fn ads_escalate(&self, id: uuid::Uuid, reason: Option<String>) -> AdminResult<()> {
        self.ads_dac_action(id, "escalate", reason).await
    }

    async fn ads_dac_action(
        &self,
        id: uuid::Uuid,
        action: &str,
        reason: Option<String>,
    ) -> AdminResult<()> {
        let body = DacActionRequest {
            reason,
            actor: Some("admin-ui".to_string()),
        };
        let resp = self
            .http
            .post(self.ads_url(&format!("/dac/requests/{id}/{action}")))
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

    pub async fn ads_create_dataset(&self, payload: &CreateDatasetRequest) -> AdminResult<Dataset> {
        self.ads_post("/datasets", payload).await
    }

    pub async fn ads_create_project(
        &self,
        payload: &CreateProjectRequest,
    ) -> AdminResult<ResearchProject> {
        self.ads_post("/projects", payload).await
    }

    pub async fn ads_create_permission_source(
        &self,
        payload: &CreatePermissionSourceRequest,
    ) -> AdminResult<PermissionSource> {
        self.ads_post("/permission-sources", payload).await
    }

    pub async fn ads_create_permission_mapping(
        &self,
        payload: &CreatePermissionMappingRequest,
    ) -> AdminResult<PermissionMapping> {
        self.ads_post("/permission-mappings", payload).await
    }

    pub async fn ads_delete_permission_mapping(&self, id: uuid::Uuid) -> AdminResult<()> {
        self.ads_delete(&format!("/permission-mappings/{id}")).await
    }

    pub async fn ads_revoke_grant(&self, id: uuid::Uuid) -> AdminResult<()> {
        self.ads_delete(&format!("/grants/{id}")).await
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

    pub async fn registry_list_services(&self) -> AdminResult<Vec<RegistryService>> {
        let url = format!(
            "{}/services",
            self.config.service_registry_base_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "Service registry returned {}",
                resp.status()
            )));
        }
        resp.json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
    }

    pub async fn registry_delete_service(&self, id: &str) -> AdminResult<()> {
        let key = self
            .config
            .service_registry_registration_key
            .as_deref()
            .ok_or(AdminUiError::Forbidden)?;
        let url = format!(
            "{}/services/{}",
            self.config.service_registry_base_url.trim_end_matches('/'),
            id
        );
        let resp = self
            .http
            .delete(&url)
            .header("X-API-Key", key)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "Service registry delete returned {}",
                resp.status()
            )));
        }
        Ok(())
    }

    pub async fn agreement_list_templates(&self) -> AdminResult<Vec<AgreementTemplate>> {
        let url = format!(
            "{}/templates",
            self.config
                .agreement_registry_base_url
                .trim_end_matches('/')
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AdminUiError::Upstream(format!(
                "Agreement registry returned {}",
                resp.status()
            )));
        }
        let body: AgreementTemplateListResponse = resp
            .json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        Ok(body.templates)
    }

    pub async fn agreement_compatibility_check(
        &self,
        payload: &CompatibilityCheckRequest,
    ) -> AdminResult<CompatibilityCheckResult> {
        let url = format!(
            "{}/compatibility-check",
            self.config
                .agreement_registry_base_url
                .trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .json(payload)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AdminUiError::BadRequest(format!(
                "compatibility check failed: {status} {text}"
            )));
        }
        resp.json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
    }

    pub async fn broker_openid_config(&self) -> AdminResult<Value> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            self.config.broker_base_url.trim_end_matches('/')
        );
        self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
    }

    pub async fn broker_jwks(&self) -> AdminResult<Value> {
        let url = format!(
            "{}/jwks.json",
            self.config.broker_base_url.trim_end_matches('/')
        );
        self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| AdminUiError::Upstream(e.to_string()))
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

#[derive(Debug, Clone, Deserialize)]
pub struct DuoTermOption {
    pub code: String,
    pub obo_id: String,
    pub label: String,
    pub definition: String,
    pub category: String,
    pub obsolete: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryService {
    pub url: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub r#type: Option<ga4gh_types::ServiceType>,
}

impl RegistryService {
    pub fn version_label(&self) -> String {
        if !self.version.is_empty() {
            return self.version.clone();
        }
        self.r#type
            .as_ref()
            .map(|t| t.version.clone())
            .unwrap_or_else(|| "—".into())
    }
}
