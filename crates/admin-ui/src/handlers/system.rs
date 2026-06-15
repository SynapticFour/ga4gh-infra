use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, Response};
use axum::Form;
use ga4gh_types::{CreatePermissionMappingRequest, CreatePermissionSourceRequest};
use serde::Deserialize;
use uuid::Uuid;

use crate::handlers::{render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "system/index.html")]
struct SystemInner {
    pub broker_config_path: String,
    pub oidc_config: Option<String>,
    pub jwks: Option<String>,
    pub sources: Vec<SourceRow>,
    pub mappings: Vec<MappingRow>,
    pub degraded: bool,
}

pub struct SourceRow {
    pub id: String,
    pub name: String,
    pub issuer: String,
    pub claim_path: String,
}

pub struct MappingRow {
    pub id: String,
    pub source_id: String,
    pub claim_value: String,
    pub dataset_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SourceForm {
    pub name: String,
    pub oidc_issuer: String,
    pub claim_path: String,
}

#[derive(Debug, Deserialize)]
pub struct MappingForm {
    pub source_id: String,
    pub claim_value: String,
    pub dataset_id: String,
    pub grant_lifetime_seconds: Option<u64>,
}

pub async fn index_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err().into_response();
    }
    render_system(&auth, &state).await
}

async fn render_system(auth: &RequireAuth, state: &SharedState) -> Response {
    let oidc_config = state
        .clients
        .broker_openid_config()
        .await
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok());
    let jwks = state
        .clients
        .broker_jwks()
        .await
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok());
    let sources_result = state.clients.ads_list_permission_sources().await;
    let mappings_result = state.clients.ads_list_permission_mappings().await;
    let degraded = sources_result.is_err() || mappings_result.is_err();
    let inner = SystemInner {
        broker_config_path: state.config.broker_config_path.clone(),
        oidc_config,
        jwks,
        sources: sources_result
            .unwrap_or_default()
            .iter()
            .map(|s| SourceRow {
                id: s.id.to_string(),
                name: s.name.clone(),
                issuer: s.oidc_issuer.clone(),
                claim_path: s.claim_path.clone(),
            })
            .collect(),
        mappings: mappings_result
            .unwrap_or_default()
            .iter()
            .map(|m| MappingRow {
                id: m.id.to_string(),
                source_id: m.source_id.to_string(),
                claim_value: m.claim_value.clone(),
                dataset_id: m.dataset_id.to_string(),
            })
            .collect(),
        degraded,
    };
    match render_layout("System", "system", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_source(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Form(form): Form<SourceForm>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    let payload = CreatePermissionSourceRequest {
        name: form.name,
        oidc_issuer: form.oidc_issuer,
        claim_path: form.claim_path,
    };
    let _ = state.clients.ads_create_permission_source(&payload).await;
    render_system(&auth, &state).await
}

pub async fn create_mapping(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Form(form): Form<MappingForm>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    let Ok(source_id) = Uuid::parse_str(&form.source_id) else {
        return (StatusCode::BAD_REQUEST, "invalid source_id").into_response();
    };
    let Ok(dataset_id) = Uuid::parse_str(&form.dataset_id) else {
        return (StatusCode::BAD_REQUEST, "invalid dataset_id").into_response();
    };
    let payload = CreatePermissionMappingRequest {
        source_id,
        claim_value: form.claim_value,
        dataset_id,
        grant_lifetime_seconds: form.grant_lifetime_seconds,
    };
    let _ = state.clients.ads_create_permission_mapping(&payload).await;
    render_system(&auth, &state).await
}

pub async fn delete_mapping(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    let _ = state.clients.ads_delete_permission_mapping(id).await;
    render_system(&auth, &state).await
}
