use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, Response};
use ga4gh_types::AccessRequest;
use uuid::Uuid;

use crate::handlers::{htmx_redirect, is_htmx, render_layout, SharedState};
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
    pub status: String,
    pub created_at: String,
}

impl From<&AccessRequest> for QueueRow {
    fn from(r: &AccessRequest) -> Self {
        Self {
            id: r.id.to_string(),
            requester: r.researcher_id.clone(),
            dataset_id: r.dataset_id.to_string(),
            status: format!("{:?}", r.status),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

pub async fn queue_page(
    auth: RequireAuth,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let degraded = state.clients.ads_dac_queue().await.is_err();
    let inner = QueuePageInner { degraded };

    match render_layout("DAC Queue", "dac", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

pub async fn queue_partial(
    _auth: RequireAuth,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    match state.clients.ads_dac_queue().await {
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
        Err(err) => (StatusCode::SERVICE_UNAVAILABLE, err.to_string()).into_response(),
    }
}

pub async fn approve(
    _auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    dac_action_response(state.clients.ads_approve(id).await, &headers).await
}

pub async fn reject(
    _auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    dac_action_response(state.clients.ads_reject(id).await, &headers).await
}

pub async fn escalate(
    _auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    dac_action_response(state.clients.ads_escalate(id).await, &headers).await
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
