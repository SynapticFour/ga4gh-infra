use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{Html, Response};
use ga4gh_types::AdsEvent;

use crate::csv;
use crate::handlers::{render_layout, SharedState};
use crate::roles::operator_dac_groups;
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
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    let result = state.clients.ads_list_audit(100, groups.as_deref()).await;
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

pub async fn export_csv(auth: RequireAuth, State(state): State<SharedState>) -> Response {
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    let events = match state.clients.ads_list_audit(500, groups.as_deref()).await {
        Ok(events) => events,
        Err(err) => return (StatusCode::SERVICE_UNAVAILABLE, err.to_string()).into_response(),
    };

    let mut body = csv::row(&["id", "event_type", "occurred_at", "payload"]);
    for event in &events {
        let payload = serde_json::to_string(&event.payload).unwrap_or_default();
        body.push_str(&csv::row(&[
            &event.id.to_string(),
            &format!("{:?}", event.event_type),
            &event.occurred_at.to_rfc3339(),
            &payload,
        ]));
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"audit-events.csv\"",
            ),
        ],
        body,
    )
        .into_response()
}
