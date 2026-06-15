use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::response::Html;

use crate::handlers::{render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "dashboard/index.html")]
struct DashboardInner {
    pending_count: Option<usize>,
    dataset_count: Option<usize>,
    ads_ok: bool,
    duo_ok: bool,
    broker_ok: bool,
    visa_ok: bool,
    registry_ok: bool,
    activity_note: &'static str,
}

pub async fn dashboard(
    auth: RequireAuth,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let pending_count = state.clients.ads_dac_queue().await.ok().map(|q| q.len());
    let dataset_count = state.clients.ads_list_datasets().await.ok().map(|d| d.len());

    let ads_ok = state.clients.service_info_ok(&state.config.ads_base_url).await;
    let duo_ok = state.clients.service_info_ok(&state.config.duo_base_url).await;
    let broker_ok = state
        .clients
        .service_info_ok(&state.config.broker_base_url)
        .await;
    let visa_ok = state
        .clients
        .service_info_ok(&state.config.visa_registry_base_url)
        .await;
    let registry_ok = state
        .clients
        .service_info_ok(&state.config.service_registry_base_url)
        .await;

    let inner = DashboardInner {
        pending_count,
        dataset_count,
        ads_ok,
        duo_ok,
        broker_ok,
        visa_ok,
        registry_ok,
        activity_note: "Audit event listing is not yet exposed via ADS REST; recent activity will appear here in a future release.",
    };

    match render_layout("Dashboard", "dashboard", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(err) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}
