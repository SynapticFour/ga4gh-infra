use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, Response};
use axum::Form;
use ga4gh_types::{AgreementTemplate, CompatibilityCheckRequest, CompatibilityCheckResult};
use serde::Deserialize;

use crate::handlers::{render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "agreements/index.html")]
struct AgreementsInner {
    pub templates: Vec<TemplateRow>,
    pub degraded: bool,
    pub check_result: Option<CheckResultRow>,
    pub error: Option<String>,
}

pub struct TemplateRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub illustrative: bool,
}

impl From<&AgreementTemplate> for TemplateRow {
    fn from(t: &AgreementTemplate) -> Self {
        Self {
            id: t.id.clone(),
            name: t.name.clone(),
            version: t.version.clone(),
            description: t.description.clone(),
            illustrative: t.is_illustrative,
        }
    }
}

pub struct CheckResultRow {
    pub compatible: bool,
    pub matched_template: String,
    pub unsatisfied: String,
    pub conditions: String,
    pub decision_record_id: String,
}

impl From<&CompatibilityCheckResult> for CheckResultRow {
    fn from(r: &CompatibilityCheckResult) -> Self {
        Self {
            compatible: r.compatible,
            matched_template: r.matched_template.clone().unwrap_or_else(|| "—".into()),
            unsatisfied: r
                .unsatisfied_codes
                .iter()
                .map(|c| c.as_str().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            conditions: r.conditions.join("; "),
            decision_record_id: r.decision_record_id.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CompatibilityForm {
    pub requester_profile_id: String,
    pub dataset_profile_id: String,
}

async fn agreements_page(
    auth: &RequireAuth,
    state: &SharedState,
    check_result: Option<CheckResultRow>,
    error: Option<String>,
) -> Response {
    let templates_result = state.clients.agreement_list_templates().await;
    let inner = AgreementsInner {
        templates: templates_result
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(TemplateRow::from)
            .collect(),
        degraded: templates_result.is_err(),
        check_result,
        error,
    };
    match render_layout("Agreements", "agreements", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn index_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err().into_response();
    }
    agreements_page(&auth, &state, None, None).await
}

pub async fn compatibility_check(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Form(form): Form<CompatibilityForm>,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err().into_response();
    }
    let payload = CompatibilityCheckRequest {
        requester_profile_id: form.requester_profile_id,
        dataset_profile_id: form.dataset_profile_id,
    };
    match state.clients.agreement_compatibility_check(&payload).await {
        Ok(result) => {
            agreements_page(&auth, &state, Some(CheckResultRow::from(&result)), None).await
        }
        Err(err) => agreements_page(&auth, &state, None, Some(err.to_string())).await,
    }
}
