use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, Redirect, Response};
use axum::Form;
use ga4gh_types::{CreateDatasetRequest, Dataset, DuoCode};
use serde::Deserialize;
use uuid::Uuid;

use crate::clients::DuoTermOption;
use crate::duo::duo_label;
use crate::handlers::{render_layout, SharedState};
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
    pub duo_labels: Vec<String>,
}

pub struct DatasetRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub external_id: String,
    pub duo_codes: String,
    pub auto_approve: String,
    pub created_at: String,
}

impl From<&Dataset> for DatasetRow {
    fn from(d: &Dataset) -> Self {
        let duo_codes = d
            .duo_codes
            .iter()
            .map(|c| c.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            id: d.id.to_string(),
            name: d.name.clone(),
            description: d.description.clone().unwrap_or_default(),
            external_id: d.external_id.clone().unwrap_or_default(),
            duo_codes,
            auto_approve: if d.auto_approve_enabled {
                format!("yes (threshold {})", d.auto_approve_threshold)
            } else {
                "no".to_string()
            },
            created_at: d.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDatasetForm {
    pub name: String,
    pub description: Option<String>,
    pub external_id: Option<String>,
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

pub async fn list_page(
    auth: RequireAuth,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let datasets_result = state.clients.ads_list_datasets().await;
    let duo_terms = state.clients.duo_terms().await.unwrap_or_default();
    let degraded = datasets_result.is_err();
    let datasets: Vec<DatasetRow> = datasets_result
        .unwrap_or_default()
        .iter()
        .map(DatasetRow::from)
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
    match state.clients.ads_get_dataset(id).await {
        Ok(dataset) => {
            let duo_labels: Vec<String> = dataset
                .duo_codes
                .iter()
                .map(|c| duo_label(c.obo_id()))
                .collect();
            let inner = DetailInner {
                dataset: DatasetRow::from(&dataset),
                duo_labels,
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
    };

    match state.clients.ads_create_dataset(&payload).await {
        Ok(dataset) => Redirect::to(&format!("/datasets/{}", dataset.id)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}
