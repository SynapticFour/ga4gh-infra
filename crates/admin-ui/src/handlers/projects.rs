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
use crate::datetime::FormattedDateTime;
use crate::duo::{duo_display, DuoDisplay};
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
    pub duo_terms_detail: Vec<DuoTermDetail>,
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

pub struct ProjectRow {
    pub id: String,
    pub researcher_id: String,
    pub name: String,
    pub description: String,
    pub duo_codes: String,
    pub created_at: FormattedDateTime,
}

impl ProjectRow {
    fn from_project(p: &ResearchProject) -> Self {
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
            created_at: FormattedDateTime::from_utc(p.created_at),
        }
    }
}

#[allow(dead_code)]
impl From<&ResearchProject> for ProjectRow {
    fn from(p: &ResearchProject) -> Self {
        ProjectRow::from_project(p)
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
    let duo_terms = state.clients.duo_terms().await.unwrap_or_default();
    match state.clients.ads_get_project(id).await {
        Ok(project) => {
            let inner = DetailInner {
                project: ProjectRow::from(&project),
                duo_terms_detail: project
                    .duo_codes
                    .iter()
                    .map(|c| DuoTermDetail::from(duo_display(c.as_str(), &duo_terms)))
                    .collect(),
                is_admin: auth.0.is_admin,
                duo_terms,
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
    match save_project(&state, None, form).await {
        Ok(p) => Redirect::to(&format!("/projects/{}", p.id)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn update(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    Form(form): Form<CreateProjectForm>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    match save_project(&state, Some(id), form).await {
        Ok(p) => Redirect::to(&format!("/projects/{}", p.id)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn save_project(
    state: &SharedState,
    id: Option<Uuid>,
    form: CreateProjectForm,
) -> crate::error::AdminResult<ResearchProject> {
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
    if let Some(id) = id {
        state.clients.ads_update_project(id, &payload).await
    } else {
        state.clients.ads_create_project(&payload).await
    }
}
