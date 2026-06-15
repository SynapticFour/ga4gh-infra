use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, Redirect, Response};
use axum::Form;
use ga4gh_types::{CreateProjectRequest, DuoCode, ResearchProject};
use serde::Deserialize;
use uuid::Uuid;

use crate::clients::DuoTermOption;
use crate::handlers::{render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "projects/list.html")]
struct ListInner {
    pub projects: Vec<ProjectRow>,
    pub degraded: bool,
    pub is_admin: bool,
    pub duo_terms: Vec<DuoTermOption>,
}

#[derive(Template)]
#[template(path = "projects/detail.html")]
struct DetailInner {
    pub project: ProjectRow,
    pub duo_labels: Vec<String>,
}

pub struct ProjectRow {
    pub id: String,
    pub researcher_id: String,
    pub name: String,
    pub description: String,
    pub duo_codes: String,
    pub created_at: String,
}

impl From<&ResearchProject> for ProjectRow {
    fn from(p: &ResearchProject) -> Self {
        Self {
            id: p.id.to_string(),
            researcher_id: p.researcher_id.clone(),
            name: p.name.clone(),
            description: p.description.clone().unwrap_or_default(),
            duo_codes: p
                .duo_codes
                .iter()
                .map(|c| c.as_str().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectForm {
    pub researcher_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub duo_codes: Vec<String>,
}

pub async fn list_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let result = state.clients.ads_list_projects().await;
    let inner = ListInner {
        projects: result
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(ProjectRow::from)
            .collect(),
        degraded: result.is_err(),
        is_admin: auth.0.is_admin,
        duo_terms: state.clients.duo_terms().await.unwrap_or_default(),
    };
    match render_layout("Projects", "projects", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn detail_page(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.clients.ads_get_project(id).await {
        Ok(project) => {
            let inner = DetailInner {
                project: ProjectRow::from(&project),
                duo_labels: project
                    .duo_codes
                    .iter()
                    .map(|c| crate::duo::duo_label(c.obo_id()))
                    .collect(),
            };
            match render_layout("Project", "projects", &auth.0, inner) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
        Err(crate::error::AdminUiError::NotFound) => {
            (StatusCode::NOT_FOUND, "project not found").into_response()
        }
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
    }
}

pub async fn create(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Form(form): Form<CreateProjectForm>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    let duo_codes: Vec<DuoCode> = form
        .duo_codes
        .iter()
        .filter_map(|c| c.parse().ok())
        .collect();
    let payload = CreateProjectRequest {
        researcher_id: form.researcher_id,
        name: form.name,
        description: form.description.filter(|s| !s.is_empty()),
        duo_codes,
    };
    match state.clients.ads_create_project(&payload).await {
        Ok(p) => Redirect::to(&format!("/projects/{}", p.id)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
