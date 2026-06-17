// SPDX-License-Identifier: Apache-2.0

//! Seed datasets, projects, DAC queue items, grants, services, and visas.

use std::time::Duration;

use anyhow::{Context, Result};
use ga4gh_types::{
    AccessRequest, AdsResourceType, CreateDatasetRequest, CreateProjectRequest, Dataset,
    DatasetVisibility, DuoCode, GrantListResponse, ResearchProject,
};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, SET_COOKIE};
use reqwest::{Client, StatusCode};
use serde_json::json;
use uuid::Uuid;

use crate::config::SeedConfig;

pub const SEED_MARKER: &str = "dev-stack-seed-v1";

#[derive(Debug, Default)]
pub struct SeedSummary {
    pub services_registered: usize,
    pub datasets_created: usize,
    pub datasets_skipped: usize,
    pub projects_created: usize,
    pub projects_skipped: usize,
    pub pending_requests_created: usize,
    pub pending_requests_skipped: usize,
    pub grants_created: usize,
    pub grants_skipped: usize,
    pub visas_created: usize,
    pub visas_skipped: usize,
}

pub async fn seed_dev_stack(config: &SeedConfig) -> Result<SeedSummary> {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .context("build HTTP client")?;

    wait_for_service(&client, &config.ads_url).await?;
    wait_for_service(&client, &config.broker_url).await?;
    wait_for_service(&client, &config.service_registry_url).await?;
    wait_for_service(&client, &config.visa_registry_url).await?;

    let mut summary = SeedSummary {
        services_registered: seed_service_registry(&client, config).await?,
        ..Default::default()
    };

    let datasets = seed_datasets(&client, config, &mut summary).await?;
    let _compute_pool = seed_compute_pool(&client, config, &mut summary).await?;
    let passport = broker_login(&client, config).await?;
    let projects = seed_projects(&client, config, &passport, &mut summary).await?;

    seed_access_workflow(
        &client,
        config,
        &passport,
        &datasets,
        &projects,
        &mut summary,
    )
    .await?;
    seed_visas(&client, config, &mut summary).await?;

    Ok(summary)
}

async fn wait_for_service(client: &Client, base: &str) -> Result<()> {
    let url = format!("{}/service-info", base.trim_end_matches('/'));
    for attempt in 0..60 {
        if client
            .get(&url)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok(());
        }
        if attempt == 59 {
            anyhow::bail!("service at {base} did not become ready");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Ok(())
}

struct ServiceRegistration {
    id: &'static str,
    name: &'static str,
    artifact: &'static str,
    url: String,
    spec_version: &'static str,
}

async fn seed_service_registry(client: &Client, config: &SeedConfig) -> Result<usize> {
    let services = [
        ServiceRegistration {
            id: "org.localhost.aai-broker",
            name: "GA4GH AAI Broker",
            artifact: "passport",
            url: config.broker_url.clone(),
            spec_version: "1.2",
        },
        ServiceRegistration {
            id: "org.localhost.visa-registry",
            name: "GA4GH Visa Registry",
            artifact: "visa",
            url: config.visa_registry_url.clone(),
            spec_version: "1.0",
        },
        ServiceRegistration {
            id: "org.localhost.duo-service",
            name: "GA4GH DUO Service",
            artifact: "duo",
            url: match config.profile {
                crate::config::SeedProfile::Postgres => "http://localhost:8082".to_string(),
                crate::config::SeedProfile::Sqlite => "http://localhost:8182".to_string(),
            },
            spec_version: "1.0",
        },
        ServiceRegistration {
            id: "org.localhost.access-decision-service",
            name: "GA4GH Access Decision Service",
            artifact: "access-decision-service",
            url: config.ads_url.clone(),
            spec_version: "1.0.0",
        },
        ServiceRegistration {
            id: "org.localhost.sample-resource",
            name: "GA4GH Sample Resource",
            artifact: "resource",
            url: config.sample_resource_url.clone(),
            spec_version: "1.0",
        },
        ServiceRegistration {
            id: "org.localhost.agreement-registry",
            name: "GA4GH Agreement Registry",
            artifact: "agreement-registry",
            url: config.agreement_registry_url.clone(),
            spec_version: "0.1",
        },
    ];

    let mut registered = 0usize;
    for svc in services {
        let response = client
            .post(format!(
                "{}/services",
                config.service_registry_url.trim_end_matches('/')
            ))
            .header("X-API-Key", &config.service_registry_key)
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "id": svc.id,
                "name": svc.name,
                "type": {
                    "group": "org.ga4gh",
                    "artifact": svc.artifact,
                    "version": svc.spec_version
                },
                "organization": {
                    "name": "GA4GH Infra",
                    "url": "https://ga4gh.org"
                },
                "version": "0.1.0",
                "url": svc.url,
                "environment": "development"
            }))
            .send()
            .await
            .with_context(|| format!("register service {}", svc.id))?;
        if response.status().is_success() {
            registered += 1;
        } else {
            anyhow::bail!("register {} failed: HTTP {}", svc.id, response.status());
        }
    }
    Ok(registered)
}

struct SeedDataset {
    external_id: &'static str,
    name: &'static str,
    description: &'static str,
    duo_codes: &'static [&'static str],
}

async fn seed_datasets(
    client: &Client,
    config: &SeedConfig,
    summary: &mut SeedSummary,
) -> Result<std::collections::HashMap<&'static str, Dataset>> {
    let specs = [
        SeedDataset {
            external_id: "dataset-registered-access-demo",
            name: "Registered Access Demo Cohort",
            description: "Synthetic cohort wired to the sample-resource demo endpoint",
            duo_codes: &["GRU", "NPU"],
        },
        SeedDataset {
            external_id: "dataset-controlled-cohort",
            name: "Controlled Access Neuroimaging Cohort",
            description: "Requires manual DAC review in the admin UI",
            duo_codes: &["GRU", "NPU"],
        },
        SeedDataset {
            external_id: "dataset-public-summary-stats",
            name: "Public Summary Statistics",
            description: "Non-restricted summary tables for dashboard variety",
            duo_codes: &["NRES"],
        },
    ];

    let existing = list_datasets(client, config).await?;
    let mut out = std::collections::HashMap::new();

    for spec in specs {
        if let Some(dataset) = existing
            .iter()
            .find(|d| d.external_id.as_deref() == Some(spec.external_id))
        {
            summary.datasets_skipped += 1;
            out.insert(spec.external_id, dataset.clone());
            continue;
        }

        let dataset = create_dataset(client, config, &spec).await?;
        summary.datasets_created += 1;
        out.insert(spec.external_id, dataset);
    }

    Ok(out)
}

async fn list_datasets(client: &Client, config: &SeedConfig) -> Result<Vec<Dataset>> {
    let response = client
        .get(format!(
            "{}/ads/v1/datasets",
            config.ads_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.ads_api_key)
        .send()
        .await
        .context("list datasets")?;
    if !response.status().is_success() {
        anyhow::bail!("list datasets returned {}", response.status());
    }
    let body = response.json::<serde_json::Value>().await?;
    Ok(serde_json::from_value(body["datasets"].clone()).unwrap_or_default())
}

async fn create_dataset(
    client: &Client,
    config: &SeedConfig,
    spec: &SeedDataset,
) -> Result<Dataset> {
    let duo_codes = spec
        .duo_codes
        .iter()
        .map(|code| code.parse::<DuoCode>())
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("parse DUO codes for {}", spec.external_id))?;

    let response = client
        .post(format!(
            "{}/ads/v1/datasets",
            config.ads_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.ads_api_key)
        .header(CONTENT_TYPE, "application/json")
        .json(&CreateDatasetRequest {
            name: spec.name.to_string(),
            description: Some(spec.description.to_string()),
            duo_codes,
            external_id: Some(spec.external_id.to_string()),
            auto_approve_enabled: false,
            auto_approve_threshold: 100,
            dac_group: Some("local-dac".to_string()),
            visibility: DatasetVisibility::Institute,
            resource_type: AdsResourceType::Dataset,
            remote_drs_base_url: None,
        })
        .send()
        .await
        .with_context(|| format!("create dataset {}", spec.external_id))?;
    if !response.status().is_success() {
        anyhow::bail!(
            "create dataset {} failed: {}",
            spec.external_id,
            response.status()
        );
    }
    response.json().await.context("dataset response json")
}

async fn seed_compute_pool(
    client: &Client,
    config: &SeedConfig,
    summary: &mut SeedSummary,
) -> Result<Dataset> {
    let external_id = "compute-pool-ferrum-tes";
    let existing = list_datasets(client, config).await?;
    if let Some(dataset) = existing
        .iter()
        .find(|d| d.external_id.as_deref() == Some(external_id))
    {
        summary.datasets_skipped += 1;
        return Ok(dataset.clone());
    }

    let response = client
        .post(format!(
            "{}/ads/v1/datasets",
            config.ads_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.ads_api_key)
        .header(CONTENT_TYPE, "application/json")
        .json(&CreateDatasetRequest {
            name: "Ferrum TES Compute Pool".to_string(),
            description: Some(
                "Shared TES/WES compute resource registered in ADS for access-controlled runs"
                    .to_string(),
            ),
            duo_codes: vec!["GRU".parse().expect("duo")],
            external_id: Some(external_id.to_string()),
            auto_approve_enabled: false,
            auto_approve_threshold: 100,
            dac_group: Some("local-dac".to_string()),
            visibility: DatasetVisibility::Institute,
            resource_type: AdsResourceType::ComputePool,
            remote_drs_base_url: None,
        })
        .send()
        .await
        .context("create compute pool resource")?;
    if !response.status().is_success() {
        anyhow::bail!("create compute pool failed: {}", response.status());
    }
    summary.datasets_created += 1;
    response.json().await.context("compute pool response json")
}

struct SeedProject {
    name: &'static str,
    description: &'static str,
    duo_codes: &'static [&'static str],
}

async fn seed_projects(
    client: &Client,
    config: &SeedConfig,
    passport: &str,
    summary: &mut SeedSummary,
) -> Result<std::collections::HashMap<&'static str, ResearchProject>> {
    let specs = [
        SeedProject {
            name: "Heidelberg Rare Disease Study",
            description: "Pending DAC review demo project",
            duo_codes: &["HMB", "NPU"],
        },
        SeedProject {
            name: "Registered Access Pilot",
            description: "Approved grant demo project",
            duo_codes: &["HMB", "NPU"],
        },
    ];

    let existing = list_projects(client, config).await?;
    let mut out = std::collections::HashMap::new();

    for spec in specs {
        if let Some(project) = existing
            .iter()
            .find(|p| p.name == spec.name && p.researcher_id == config.researcher_sub)
        {
            summary.projects_skipped += 1;
            out.insert(spec.name, project.clone());
            continue;
        }

        let duo_codes = spec
            .duo_codes
            .iter()
            .map(|code| code.parse::<DuoCode>())
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("parse DUO codes for project {}", spec.name))?;

        let response = client
            .post(format!(
                "{}/ads/v1/projects",
                config.ads_url.trim_end_matches('/')
            ))
            .header(AUTHORIZATION, format!("Bearer {passport}"))
            .header(CONTENT_TYPE, "application/json")
            .json(&CreateProjectRequest {
                researcher_id: config.researcher_sub.clone(),
                name: spec.name.to_string(),
                description: Some(spec.description.to_string()),
                duo_codes,
            })
            .send()
            .await
            .with_context(|| format!("create project {}", spec.name))?;
        if !response.status().is_success() {
            anyhow::bail!("create project {} failed: {}", spec.name, response.status());
        }
        let project: ResearchProject = response.json().await?;
        summary.projects_created += 1;
        out.insert(spec.name, project);
    }

    Ok(out)
}

async fn list_projects(client: &Client, config: &SeedConfig) -> Result<Vec<ResearchProject>> {
    let response = client
        .get(format!(
            "{}/ads/v1/projects",
            config.ads_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.ads_api_key)
        .send()
        .await
        .context("list projects")?;
    if !response.status().is_success() {
        anyhow::bail!("list projects returned {}", response.status());
    }
    let body = response.json::<serde_json::Value>().await?;
    Ok(serde_json::from_value(body["projects"].clone()).unwrap_or_default())
}

async fn seed_access_workflow(
    client: &Client,
    config: &SeedConfig,
    passport: &str,
    datasets: &std::collections::HashMap<&'static str, Dataset>,
    projects: &std::collections::HashMap<&'static str, ResearchProject>,
    summary: &mut SeedSummary,
) -> Result<()> {
    let demo_dataset = datasets
        .get("dataset-registered-access-demo")
        .context("missing demo dataset")?;
    let controlled_dataset = datasets
        .get("dataset-controlled-cohort")
        .context("missing controlled dataset")?;
    let pending_project = projects
        .get("Heidelberg Rare Disease Study")
        .context("missing pending project")?;
    let approved_project = projects
        .get("Registered Access Pilot")
        .context("missing approved project")?;

    let pending_justification = format!("{SEED_MARKER}: pending review for admin-ui DAC queue");
    if !has_pending_request(client, config, &pending_justification).await? {
        create_access_request(
            client,
            config,
            passport,
            controlled_dataset.id,
            pending_project.id,
            &pending_justification,
        )
        .await?;
        summary.pending_requests_created += 1;
    } else {
        summary.pending_requests_skipped += 1;
    }

    if !has_active_grant(client, config, demo_dataset.id).await? {
        let request = create_access_request(
            client,
            config,
            passport,
            demo_dataset.id,
            approved_project.id,
            &format!("{SEED_MARKER}: approved grant for dashboard"),
        )
        .await?;

        if request.status != ga4gh_types::AccessRequestStatus::Approved {
            approve_request(client, config, request.id).await?;
            summary.grants_created += 1;
        } else {
            summary.grants_skipped += 1;
        }
    } else {
        summary.grants_skipped += 1;
    }

    Ok(())
}

async fn has_pending_request(
    client: &Client,
    config: &SeedConfig,
    justification: &str,
) -> Result<bool> {
    let queue = list_dac_queue(client, config).await?;
    Ok(queue
        .iter()
        .any(|request| request.justification.as_deref() == Some(justification)))
}

async fn has_active_grant(client: &Client, config: &SeedConfig, dataset_id: Uuid) -> Result<bool> {
    let grants = list_grants(client, config).await?;
    Ok(grants.grants.iter().any(|grant| {
        grant.dataset_id == dataset_id && grant.researcher_id == config.researcher_sub
    }))
}

async fn list_dac_queue(client: &Client, config: &SeedConfig) -> Result<Vec<AccessRequest>> {
    let response = client
        .get(format!(
            "{}/ads/v1/dac/requests",
            config.ads_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.ads_api_key)
        .send()
        .await
        .context("list dac queue")?;
    if !response.status().is_success() {
        anyhow::bail!("list dac queue returned {}", response.status());
    }
    let body = response.json::<serde_json::Value>().await?;
    Ok(serde_json::from_value(body["requests"].clone()).unwrap_or_default())
}

async fn list_grants(client: &Client, config: &SeedConfig) -> Result<GrantListResponse> {
    let response = client
        .get(format!(
            "{}/ads/v1/grants",
            config.ads_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.ads_api_key)
        .query(&[("researcher_id", config.researcher_sub.as_str())])
        .send()
        .await
        .context("list grants")?;
    if !response.status().is_success() {
        anyhow::bail!("list grants returned {}", response.status());
    }
    response.json().await.context("grants json")
}

async fn create_access_request(
    client: &Client,
    config: &SeedConfig,
    passport: &str,
    dataset_id: Uuid,
    project_id: Uuid,
    justification: &str,
) -> Result<AccessRequest> {
    let response = client
        .post(format!(
            "{}/ads/v1/access-requests",
            config.ads_url.trim_end_matches('/')
        ))
        .header(AUTHORIZATION, format!("Bearer {passport}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "researcher_id": config.researcher_sub,
            "dataset_id": dataset_id,
            "project_id": project_id,
            "justification": justification,
        }))
        .send()
        .await
        .context("create access request")?;
    if !response.status().is_success() {
        anyhow::bail!("create access request failed: {}", response.status());
    }
    response.json().await.context("access request json")
}

async fn approve_request(client: &Client, config: &SeedConfig, request_id: Uuid) -> Result<()> {
    let response = client
        .post(format!(
            "{}/ads/v1/dac/requests/{request_id}/approve",
            config.ads_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.ads_api_key)
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "reason": format!("{SEED_MARKER}: pre-approved demo grant")
        }))
        .send()
        .await
        .context("approve access request")?;
    if response.status() != StatusCode::OK {
        anyhow::bail!("approve request failed: {}", response.status());
    }
    Ok(())
}

async fn seed_visas(client: &Client, config: &SeedConfig, summary: &mut SeedSummary) -> Result<()> {
    let existing = client
        .get(format!(
            "{}/visas",
            config.visa_registry_url.trim_end_matches('/')
        ))
        .query(&[("sub", config.researcher_sub.as_str())])
        .send()
        .await
        .context("list visas")?;

    if existing.status().is_success() {
        let body = existing
            .json::<serde_json::Value>()
            .await
            .unwrap_or_default();
        if body["visas"]
            .as_array()
            .is_some_and(|visas| !visas.is_empty())
        {
            summary.visas_skipped += 1;
            return Ok(());
        }
    }

    let marker = format!("{SEED_MARKER}: Heidelberg University Hospital");
    let response = client
        .post(format!(
            "{}/visas",
            config.visa_registry_url.trim_end_matches('/')
        ))
        .header("X-API-Key", &config.visa_api_key)
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "sub": config.researcher_sub,
            "type": "AffiliationAndAccreditation",
            "value": marker,
            "source": "https://www.klinikum.uni-heidelberg.de",
            "expires_in_seconds": 86400
        }))
        .send()
        .await
        .context("create visa")?;
    if response.status().is_success() {
        summary.visas_created += 1;
    } else {
        anyhow::bail!("create visa failed: {}", response.status());
    }
    Ok(())
}

async fn broker_login(client: &Client, config: &SeedConfig) -> Result<String> {
    let login = client
        .get(format!("{}/login", config.broker_url.trim_end_matches('/')))
        .header(ACCEPT, "application/json")
        .send()
        .await
        .context("broker login")?;
    if !login.status().is_success() {
        anyhow::bail!("broker login failed: {}", login.status());
    }

    let session_cookie = login
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("ga4gh_broker_rp_session="))
        .context("broker session cookie")?
        .split(';')
        .next()
        .context("cookie pair")?
        .to_string();

    let auth_url = login.json::<serde_json::Value>().await?["authorization_url"]
        .as_str()
        .context("authorization_url")?
        .to_string();

    let auth_redirect = client.get(auth_url).send().await.context("authorize")?;
    if !auth_redirect.status().is_redirection() {
        anyhow::bail!(
            "authorize expected redirect, got {}",
            auth_redirect.status()
        );
    }
    let callback_url = auth_redirect
        .headers()
        .get("location")
        .context("callback location")?
        .to_str()
        .context("callback utf8")?
        .to_string();

    let callback = client
        .get(callback_url)
        .header(ACCEPT, "application/json")
        .header("Cookie", session_cookie)
        .send()
        .await
        .context("broker callback")?;
    if !callback.status().is_success() {
        anyhow::bail!("broker callback failed: {}", callback.status());
    }

    callback.json::<serde_json::Value>().await?["access_token"]
        .as_str()
        .map(str::to_string)
        .context("broker access_token")
}
