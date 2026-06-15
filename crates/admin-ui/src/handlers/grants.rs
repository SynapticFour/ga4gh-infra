use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, Response};
use ga4gh_types::Grant;
use uuid::Uuid;

use crate::handlers::{htmx_redirect, is_htmx, render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "grants/list.html")]
struct ListInner {
    pub grants: Vec<GrantRow>,
    pub degraded: bool,
    pub is_admin: bool,
}

pub struct GrantRow {
    pub id: String,
    pub researcher_id: String,
    pub dataset_id: String,
    pub source: String,
    pub status: String,
    pub issued: String,
    pub expires: String,
}

impl From<&Grant> for GrantRow {
    fn from(g: &Grant) -> Self {
        Self {
            id: g.id.to_string(),
            researcher_id: g.researcher_id.clone(),
            dataset_id: g.dataset_id.to_string(),
            source: format!("{:?}", g.source),
            status: if g.revoked_at.is_some() {
                "revoked".into()
            } else {
                "active".into()
            },
            issued: g.created_at.to_rfc3339(),
            expires: g
                .expires_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "—".into()),
        }
    }
}

pub async fn list_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let result = state.clients.ads_list_grants().await;
    let inner = ListInner {
        grants: result
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(GrantRow::from)
            .collect(),
        degraded: result.is_err(),
        is_admin: auth.0.is_admin,
    };
    match render_layout("Grants", "grants", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn revoke(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    match state.clients.ads_revoke_grant(id).await {
        Ok(()) => {
            if is_htmx(&headers) {
                let mut h = HeaderMap::new();
                htmx_redirect(&mut h, "/grants");
                (h, StatusCode::NO_CONTENT).into_response()
            } else {
                axum::response::Redirect::to("/grants").into_response()
            }
        }
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
    }
}
