use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, Response};
use chrono::{DateTime, Utc};
use ga4gh_types::AdsEvent;

use crate::csv;
use crate::datetime::FormattedDateTime;
use crate::events::format_event_label;
use crate::handlers::{render_layout, SharedState};
use crate::roles::operator_dac_groups;
use crate::session::RequireAuth;

#[derive(serde::Deserialize, Default)]
pub struct AuditFilterQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Template)]
#[template(path = "audit/list.html")]
struct ListInner {
    pub events: Vec<EventRow>,
    pub degraded: bool,
    pub from: String,
    pub to: String,
    pub entity_id: String,
}

pub struct EventRow {
    pub id: String,
    pub event_type: String,
    pub occurred_at: FormattedDateTime,
    pub summary: String,
}

impl EventRow {
    fn from_event(e: &AdsEvent, ctx: &crate::events::EventLabelContext) -> Self {
        Self {
            id: e.id.to_string(),
            event_type: format!("{:?}", e.event_type),
            occurred_at: FormattedDateTime::from_utc(e.occurred_at),
            summary: format_event_label(e, ctx),
        }
    }
}

fn filter_events(events: Vec<AdsEvent>, query: &AuditFilterQuery) -> Vec<AdsEvent> {
    let from = query
        .from
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let to = query
        .to
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let entity = query
        .entity_id
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    events
        .into_iter()
        .filter(|e| {
            if let Some(from) = from {
                if e.occurred_at < from {
                    return false;
                }
            }
            if let Some(to) = to {
                if e.occurred_at > to {
                    return false;
                }
            }
            if let Some(entity) = &entity {
                let haystack = format!(
                    "{:?} {}",
                    e.event_type,
                    serde_json::to_string(&e.payload).unwrap_or_default()
                );
                if !haystack.to_lowercase().contains(entity) {
                    return false;
                }
            }
            true
        })
        .collect()
}

pub async fn list_page(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Query(query): Query<AuditFilterQuery>,
) -> impl IntoResponse {
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    let result = state.clients.ads_list_audit(500, groups.as_deref()).await;
    let label_ctx = state
        .clients
        .event_label_context(&auth.0, &state.config.admin_claim_value)
        .await;
    let filtered = result
        .as_ref()
        .map(|events| filter_events(events.clone(), &query))
        .unwrap_or_default();
    let inner = ListInner {
        events: filtered
            .iter()
            .map(|e| EventRow::from_event(e, &label_ctx))
            .collect(),
        degraded: result.is_err(),
        from: query.from.unwrap_or_default(),
        to: query.to.unwrap_or_default(),
        entity_id: query.entity_id.unwrap_or_default(),
    };
    match render_layout("Audit Log", "audit", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn export_csv(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Query(query): Query<AuditFilterQuery>,
) -> Response {
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    let events = match state.clients.ads_list_audit(500, groups.as_deref()).await {
        Ok(events) => filter_events(events, &query),
        Err(err) => return (StatusCode::SERVICE_UNAVAILABLE, err.to_string()).into_response(),
    };
    let label_ctx = state
        .clients
        .event_label_context(&auth.0, &state.config.admin_claim_value)
        .await;

    let mut body = csv::row(&["id", "event_type", "occurred_at", "summary"]);
    for event in &events {
        body.push_str(&csv::row(&[
            &event.id.to_string(),
            &format!("{:?}", event.event_type),
            &event.occurred_at.to_rfc3339(),
            &format_event_label(event, &label_ctx),
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
