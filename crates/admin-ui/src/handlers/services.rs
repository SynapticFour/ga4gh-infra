use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, Response};

use crate::clients::RegistryService;
use crate::handlers::{htmx_redirect, is_htmx, render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "services/list.html")]
struct ListInner {
    pub services: Vec<ServiceRow>,
    pub degraded: bool,
    pub is_admin: bool,
}

pub struct ServiceRow {
    pub id: String,
    pub name: String,
    pub url: String,
    pub version: String,
}

impl From<&RegistryService> for ServiceRow {
    fn from(s: &RegistryService) -> Self {
        Self {
            id: s.id.clone(),
            name: s.name.clone(),
            url: s.url.clone(),
            version: s.version_label(),
        }
    }
}

pub async fn list_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let result = state.clients.registry_list_services().await;
    let inner = ListInner {
        services: result
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(ServiceRow::from)
            .collect(),
        degraded: result.is_err(),
        is_admin: auth.0.is_admin,
    };
    match render_layout("Service Registry", "services", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_service(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    match state.clients.registry_delete_service(&id).await {
        Ok(()) => {
            if is_htmx(&headers) {
                let mut h = HeaderMap::new();
                htmx_redirect(&mut h, "/services");
                (h, StatusCode::NO_CONTENT).into_response()
            } else {
                axum::response::Redirect::to("/services").into_response()
            }
        }
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
    }
}
