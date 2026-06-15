use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, Response};
use axum::Form;
use ga4gh_types::Researcher;
use serde::Deserialize;

use crate::handlers::{render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "researchers/search.html")]
struct SearchInner {
    pub query: String,
    pub researcher: Option<ResearcherRow>,
    pub visas_json: Option<String>,
    pub error: Option<String>,
}

pub struct ResearcherRow {
    pub id: String,
    pub display_name: String,
    pub email: String,
    pub affiliations: String,
}

impl From<&Researcher> for ResearcherRow {
    fn from(r: &Researcher) -> Self {
        Self {
            id: r.id.clone(),
            display_name: r.display_name.clone().unwrap_or_default(),
            email: r.email.clone().unwrap_or_default(),
            affiliations: r
                .affiliations
                .iter()
                .map(|a| format!("{} ({})", a.organization, a.role))
                .collect::<Vec<_>>()
                .join("; "),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchForm {
    pub researcher_id: String,
}

pub async fn search_page(auth: RequireAuth, State(_state): State<SharedState>) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    let inner = SearchInner {
        query: String::new(),
        researcher: None,
        visas_json: None,
        error: None,
    };
    match render_layout("Researchers", "researchers", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn search(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Form(form): Form<SearchForm>,
) -> impl IntoResponse {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err().into_response();
    }
    let id = form.researcher_id.trim();
    let (researcher, visas_json, error) = match state.clients.ads_get_researcher(id).await {
        Ok(r) => {
            let visas = state.clients.ads_get_researcher_visas(id).await.ok();
            let json = visas.and_then(|v| serde_json::to_string_pretty(&v).ok());
            (Some(ResearcherRow::from(&r)), json, None)
        }
        Err(crate::error::AdminUiError::NotFound) => {
            (None, None, Some("Researcher not found".into()))
        }
        Err(e) => (None, None, Some(e.to_string())),
    };
    let inner = SearchInner {
        query: id.to_string(),
        researcher,
        visas_json,
        error,
    };
    match render_layout("Researchers", "researchers", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
