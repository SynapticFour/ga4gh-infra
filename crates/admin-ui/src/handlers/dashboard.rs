use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::response::Html;

use crate::handlers::{render_layout, SharedState};
use crate::roles::operator_dac_groups;
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "dashboard/index.html")]
struct DashboardInner {
    pending_count: Option<usize>,
    dataset_count: Option<usize>,
    grant_count: Option<usize>,
    recent_events: Vec<ActivityRow>,
    events_degraded: bool,
    ads_ok: bool,
    duo_ok: bool,
    broker_ok: bool,
    visa_ok: bool,
    registry_ok: bool,
}

pub struct ActivityRow {
    pub occurred_at: String,
    pub label: String,
}

pub async fn dashboard(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    let pending_count = state
        .clients
        .ads_dac_queue(groups.as_deref())
        .await
        .ok()
        .map(|q| q.len());
    let dataset_count = state
        .clients
        .ads_list_datasets(groups.as_deref())
        .await
        .ok()
        .map(|d| d.len());
    let grant_count = state
        .clients
        .ads_list_grants(groups.as_deref())
        .await
        .ok()
        .map(|g| g.len());
    let events_result = state.clients.ads_list_audit(10, groups.as_deref()).await;
    let recent_events: Vec<ActivityRow> = events_result
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|e| ActivityRow {
            occurred_at: e.occurred_at.to_rfc3339(),
            label: format!("{:?}", e.event_type),
        })
        .collect();

    let inner = DashboardInner {
        pending_count,
        dataset_count,
        grant_count,
        recent_events,
        events_degraded: events_result.is_err(),
        ads_ok: state
            .clients
            .service_info_ok(&state.config.ads_base_url)
            .await,
        duo_ok: state
            .clients
            .service_info_ok(&state.config.duo_base_url)
            .await,
        broker_ok: state
            .clients
            .service_info_ok(&state.config.broker_base_url)
            .await,
        visa_ok: state
            .clients
            .service_info_ok(&state.config.visa_registry_base_url)
            .await,
        registry_ok: state
            .clients
            .service_info_ok(&state.config.service_registry_base_url)
            .await,
    };

    match render_layout("Dashboard", "dashboard", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(err) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            err.to_string(),
        )
            .into_response(),
    }
}
