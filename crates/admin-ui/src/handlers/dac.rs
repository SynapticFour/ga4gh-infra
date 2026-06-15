use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, Response};
use axum::Form;
use ga4gh_types::AccessRequest;
use serde::Deserialize;
use uuid::Uuid;

use crate::handlers::{htmx_redirect, is_htmx, render_layout, SharedState};
use crate::roles::operator_dac_groups;
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "dac/queue_page.html")]
pub struct QueuePageInner {
    pub degraded: bool,
}

#[derive(Template)]
#[template(path = "dac/queue_partial.html")]
pub struct QueuePartial {
    pub requests: Vec<QueueRow>,
    pub degraded: bool,
    pub message: Option<String>,
}

pub struct QueueRow {
    pub id: String,
    pub requester: String,
    pub dataset_id: String,
    pub dac_group: String,
    pub status: String,
    pub created_at: String,
}

impl From<&AccessRequest> for QueueRow {
    fn from(r: &AccessRequest) -> Self {
        Self {
            id: r.id.to_string(),
            requester: r.researcher_id.clone(),
            dataset_id: r.dataset_id.to_string(),
            dac_group: r.dac_group.clone().unwrap_or_else(|| "—".into()),
            status: format!("{:?}", r.status),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

fn dac_groups_for(auth: &RequireAuth, state: &SharedState) -> Option<Vec<String>> {
    operator_dac_groups(&auth.0, &state.config.admin_claim_value)
}

#[derive(Debug, Deserialize)]
pub struct DacActionForm {
    pub reason: Option<String>,
}

pub async fn queue_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let groups = dac_groups_for(&auth, &state);
    let degraded = state
        .clients
        .ads_dac_queue(groups.as_deref())
        .await
        .is_err();
    let inner = QueuePageInner { degraded };

    match render_layout("DAC Queue", "dac", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

pub async fn queue_partial(
    auth: RequireAuth,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let groups = dac_groups_for(&auth, &state);
    match state.clients.ads_dac_queue(groups.as_deref()).await {
        Ok(requests) => {
            let rows: Vec<QueueRow> = requests.iter().map(QueueRow::from).collect();
            QueuePartial {
                requests: rows,
                degraded: false,
                message: None,
            }
            .into_response()
        }
        Err(err) => QueuePartial {
            requests: vec![],
            degraded: true,
            message: Some(err.to_string()),
        }
        .into_response(),
    }
}

async fn dac_action_response(
    result: crate::error::AdminResult<()>,
    headers: &HeaderMap,
) -> Response {
    match result {
        Ok(()) => {
            if is_htmx(headers) {
                let mut h = HeaderMap::new();
                htmx_redirect(&mut h, "/dac");
                (h, StatusCode::NO_CONTENT).into_response()
            } else {
                axum::response::Redirect::to("/dac").into_response()
            }
        }
        Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}

pub async fn approve(
    _auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Form(form): Form<DacActionForm>,
) -> Response {
    let reason = form.reason.filter(|s| !s.trim().is_empty());
    dac_action_response(state.clients.ads_approve(id, reason).await, &headers).await
}

pub async fn reject(
    _auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Form(form): Form<DacActionForm>,
) -> Response {
    let reason = form.reason.filter(|s| !s.trim().is_empty());
    if reason.is_none() {
        return (StatusCode::BAD_REQUEST, "reason is required for reject").into_response();
    }
    dac_action_response(state.clients.ads_reject(id, reason).await, &headers).await
}

pub async fn escalate(
    _auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Form(form): Form<DacActionForm>,
) -> Response {
    let reason = form.reason.filter(|s| !s.trim().is_empty());
    if reason.is_none() {
        return (StatusCode::BAD_REQUEST, "reason is required for escalate").into_response();
    }
    dac_action_response(state.clients.ads_escalate(id, reason).await, &headers).await
}

#[cfg(test)]
mod tests {
    use super::QueuePartial;
    use askama::Template;

    #[test]
    fn degraded_queue_partial_renders_message() {
        let tpl = QueuePartial {
            requests: vec![],
            degraded: true,
            message: Some("ADS DAC queue returned 503".to_string()),
        };
        let html = tpl.render().expect("render degraded partial");
        assert!(html.contains("Service unavailable"));
        assert!(html.contains("503"));
    }
}
