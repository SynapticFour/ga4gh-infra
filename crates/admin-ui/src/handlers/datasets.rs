use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, Redirect, Response};
use axum::Form;
use ga4gh_types::{
    AdsResourceType, CreateDatasetRequest, Dataset, DatasetVisibility, DuoCode, Grant,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::clients::DuoTermOption;
use crate::datetime::FormattedDateTime;
use crate::duo::{duo_display, DuoDisplay};
use crate::handlers::{render_layout, SharedState};
use crate::roles::operator_dac_groups;
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "datasets/list.html")]
struct ListInner {
    pub datasets: Vec<DatasetRow>,
    pub degraded: bool,
    pub is_admin: bool,
    pub duo_terms: Vec<DuoTermOption>,
}

#[derive(Template)]
#[template(path = "datasets/detail.html")]
struct DetailInner {
    pub dataset: DatasetRow,
    pub duo_terms_detail: Vec<DuoTermDetail>,
    pub active_grants: Vec<GrantRow>,
    pub policy_profile_id: Option<String>,
    pub grants_degraded: bool,
    pub is_admin: bool,
    pub duo_terms: Vec<DuoTermOption>,
}

pub struct DuoTermDetail {
    pub label: String,
    pub curie: String,
    pub definition: String,
}

impl From<DuoDisplay> for DuoTermDetail {
    fn from(d: DuoDisplay) -> Self {
        Self {
            label: d.label,
            curie: d.curie,
            definition: d.definition,
        }
    }
}

pub struct GrantRow {
    pub researcher_id: String,
    pub status: String,
    pub issued: FormattedDateTime,
}

pub struct DatasetRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub external_id: String,
    pub duo_codes_html: String,
    pub auto_approve: String,
    pub dac_group: String,
    pub grant_count: String,
    pub created_at: FormattedDateTime,
}

fn duo_codes_html(codes: &[DuoCode], terms: &[DuoTermOption]) -> String {
    codes
        .iter()
        .map(|c| {
            let d = duo_display(c.as_str(), terms);
            format!(
                r#"<span class="duo-tag" title="{}">{}</span>"#,
                html_escape(&d.curie),
                html_escape(&d.label)
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl DatasetRow {
    fn from_dataset(d: &Dataset, terms: &[DuoTermOption], grant_count: usize) -> Self {
        Self {
            id: d.id.to_string(),
            name: d.name.clone(),
            description: d.description.clone().unwrap_or_default(),
            external_id: d.external_id.clone().unwrap_or_default(),
            duo_codes_html: duo_codes_html(&d.duo_codes, terms),
            auto_approve: if d.auto_approve_enabled {
                format!("yes (threshold {})", d.auto_approve_threshold)
            } else {
                "no".to_string()
            },
            dac_group: d.dac_group.clone().unwrap_or_else(|| "—".into()),
            grant_count: grant_count.to_string(),
            created_at: FormattedDateTime::from_utc(d.created_at),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDatasetForm {
    pub name: String,
    pub description: Option<String>,
    pub external_id: Option<String>,
    pub dac_group: Option<String>,
    #[serde(default)]
    pub duo_codes: Vec<String>,
    #[serde(default)]
    pub auto_approve_enabled: bool,
    #[serde(default = "default_threshold")]
    pub auto_approve_threshold: u8,
}

fn default_threshold() -> u8 {
    100
}

fn grant_counts(grants: &[Grant]) -> HashMap<Uuid, usize> {
    let mut counts = HashMap::new();
    for g in grants {
        if g.revoked_at.is_none() {
            *counts.entry(g.dataset_id).or_insert(0) += 1;
        }
    }
    counts
}

async fn find_policy_profile(state: &SharedState, dataset: &Dataset) -> Option<String> {
    let profiles = state.clients.agreement_list_profiles().await.ok()?;
    let external = dataset.external_id.as_deref().unwrap_or("");
    profiles
        .into_iter()
        .find(|p| p.owner == external || p.id.contains(external))
        .map(|p| p.id)
}

pub async fn list_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    let datasets_result = state.clients.ads_list_datasets(groups.as_deref()).await;
    let duo_terms = state.clients.duo_terms().await.unwrap_or_default();
    let grants = state
        .clients
        .ads_list_grants(None, groups.as_deref())
        .await
        .unwrap_or_default();
    let counts = grant_counts(&grants);
    let degraded = datasets_result.is_err();
    let datasets: Vec<DatasetRow> = datasets_result
        .unwrap_or_default()
        .iter()
        .map(|d| DatasetRow::from_dataset(d, &duo_terms, *counts.get(&d.id).unwrap_or(&0)))
        .collect();

    let inner = ListInner {
        datasets,
        degraded,
        is_admin: auth.0.is_admin,
        duo_terms,
    };

    match render_layout("Datasets", "datasets", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

pub async fn detail_page(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let duo_terms = state.clients.duo_terms().await.unwrap_or_default();
    match state.clients.ads_get_dataset(id).await {
        Ok(dataset) => {
            let duo_terms_detail: Vec<DuoTermDetail> = dataset
                .duo_codes
                .iter()
                .map(|c| DuoTermDetail::from(duo_display(c.as_str(), &duo_terms)))
                .collect();
            let grants_result = state.clients.ads_list_grants(None, None).await;
            let active_grants: Vec<GrantRow> = grants_result
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .filter(|g| g.dataset_id == id && g.revoked_at.is_none())
                .map(|g| GrantRow {
                    researcher_id: g.researcher_id.clone(),
                    status: "active".into(),
                    issued: FormattedDateTime::from_utc(g.created_at),
                })
                .collect();
            let policy_profile_id = find_policy_profile(&state, &dataset).await;
            let inner = DetailInner {
                dataset: DatasetRow::from_dataset(&dataset, &duo_terms, active_grants.len()),
                duo_terms_detail,
                active_grants,
                policy_profile_id,
                grants_degraded: grants_result.is_err(),
                is_admin: auth.0.is_admin,
                duo_terms,
            };
            match render_layout("Dataset", "datasets", &auth.0, inner) {
                Ok(html) => Html(html).into_response(),
                Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
            }
        }
        Err(crate::error::AdminUiError::NotFound) => {
            (StatusCode::NOT_FOUND, "dataset not found").into_response()
        }
        Err(err) => (StatusCode::SERVICE_UNAVAILABLE, err.to_string()).into_response(),
    }
}

pub async fn create(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Form(form): Form<CreateDatasetForm>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    match save_dataset(&state, None, form).await {
        Ok(dataset) => Redirect::to(&format!("/datasets/{}", dataset.id)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}

pub async fn update(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    Form(form): Form<CreateDatasetForm>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    match save_dataset(&state, Some(id), form).await {
        Ok(dataset) => Redirect::to(&format!("/datasets/{}", dataset.id)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}

async fn save_dataset(
    state: &SharedState,
    id: Option<Uuid>,
    form: CreateDatasetForm,
) -> crate::error::AdminResult<Dataset> {
    let duo_codes: Vec<DuoCode> = form
        .duo_codes
        .iter()
        .filter_map(|c| c.parse().ok())
        .collect();
    let payload = CreateDatasetRequest {
        name: form.name,
        description: form.description.filter(|s| !s.is_empty()),
        duo_codes,
        external_id: form.external_id.filter(|s| !s.is_empty()),
        auto_approve_enabled: form.auto_approve_enabled,
        auto_approve_threshold: form.auto_approve_threshold,
        dac_group: form.dac_group.filter(|s| !s.is_empty()),
        visibility: DatasetVisibility::Institute,
        resource_type: AdsResourceType::Dataset,
        remote_drs_base_url: None,
    };
    if let Some(id) = id {
        state.clients.ads_update_dataset(id, &payload).await
    } else {
        state.clients.ads_create_dataset(&payload).await
    }
}
