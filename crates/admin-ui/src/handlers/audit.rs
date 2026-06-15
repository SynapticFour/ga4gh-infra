use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use ga4gh_types::AdsEvent;

use crate::handlers::{render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "audit/list.html")]
struct ListInner {
    pub events: Vec<EventRow>,
    pub degraded: bool,
}

pub struct EventRow {
    pub id: String,
    pub event_type: String,
    pub occurred_at: String,
    pub summary: String,
}

impl From<&AdsEvent> for EventRow {
    fn from(e: &AdsEvent) -> Self {
        let summary = serde_json::to_string(&e.payload).unwrap_or_default();
        Self {
            id: e.id.to_string(),
            event_type: format!("{:?}", e.event_type),
            occurred_at: e.occurred_at.to_rfc3339(),
            summary,
        }
    }
}

pub async fn list_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let result = state.clients.ads_list_audit(100).await;
    let inner = ListInner {
        events: result
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(EventRow::from)
            .collect(),
        degraded: result.is_err(),
    };
    match render_layout("Audit Log", "audit", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
