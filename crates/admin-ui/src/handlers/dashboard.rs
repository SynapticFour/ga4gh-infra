use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::response::Html;
use chrono::Utc;

use crate::datetime::FormattedDateTime;
use crate::events::{format_event_label, format_relative_time};
use crate::handlers::{render_layout, SharedState};
use crate::health::signing_key_summary;
use crate::roles::operator_dac_groups;
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "dashboard/index.html")]
struct DashboardInner {
    pending_count: Option<usize>,
    show_dac_link: bool,
    dataset_count: Option<usize>,
    grant_count: Option<usize>,
    service_count: Option<usize>,
    recent_events: Vec<ActivityRow>,
    events_degraded: bool,
    services: Vec<HealthRow>,
    is_admin: bool,
    signing_kid: Option<String>,
    signing_algorithm: Option<String>,
    signing_rotation_warning: bool,
    signing_rotation_due: Option<String>,
}

pub struct ActivityRow {
    pub occurred_at: FormattedDateTime,
    pub relative: String,
    pub label: String,
}

pub struct HealthRow {
    pub name: String,
    pub status_class: String,
    pub status_label: String,
    pub version: String,
    pub detail: String,
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
    let grant_count = if auth.0.is_admin {
        state
            .clients
            .ads_list_grants(None, None)
            .await
            .ok()
            .map(|g| g.len())
    } else {
        state
            .clients
            .ads_list_grants_for_operator(&auth.0.sub, groups.as_deref())
            .await
            .ok()
            .map(|g| g.len())
    };
    let service_count = state
        .clients
        .registry_list_services()
        .await
        .ok()
        .map(|s| s.len());
    let events_result = state.clients.ads_list_audit(10, groups.as_deref()).await;
    let label_ctx = state
        .clients
        .event_label_context(&auth.0, &state.config.admin_claim_value)
        .await;
    let now = Utc::now();
    let recent_events: Vec<ActivityRow> = events_result
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|e| ActivityRow {
            occurred_at: FormattedDateTime::from_utc(e.occurred_at),
            relative: format_relative_time(e.occurred_at, now),
            label: format_event_label(e, &label_ctx),
        })
        .collect();

    let health = state.clients.probe_all_services().await;
    let services: Vec<HealthRow> = health
        .into_iter()
        .map(|h| HealthRow {
            name: h.name,
            status_class: h.status.css_class().to_string(),
            status_label: h.status.label().to_string(),
            version: h.version,
            detail: h.detail.unwrap_or_default(),
        })
        .collect();

    let (signing_kid, signing_algorithm, signing_rotation_warning) = if auth.0.is_admin {
        let jwks = state.clients.broker_jwks().await.ok();
        let summary = jwks.as_ref().and_then(signing_key_summary);
        let rotation_warning = state
            .config
            .signing_key_rotation_due
            .as_ref()
            .and_then(|due| chrono::DateTime::parse_from_rfc3339(due).ok())
            .map(|due| {
                let days = (due.with_timezone(&Utc) - Utc::now()).num_days();
                days <= 30
            })
            .unwrap_or(false);
        (
            summary.as_ref().map(|s| s.kid.clone()),
            summary.as_ref().map(|s| s.algorithm.clone()),
            rotation_warning,
        )
    } else {
        (None, None, false)
    };

    let inner = DashboardInner {
        pending_count,
        show_dac_link: pending_count.is_some_and(|n| n > 0),
        dataset_count,
        grant_count,
        service_count,
        recent_events,
        events_degraded: events_result.is_err(),
        services,
        is_admin: auth.0.is_admin,
        signing_kid,
        signing_algorithm,
        signing_rotation_warning,
        signing_rotation_due: state.config.signing_key_rotation_due.clone(),
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

#[cfg(test)]
mod tests {
    use super::DashboardInner;
    use askama::Template;

    #[test]
    fn degraded_dashboard_renders_without_crashing() {
        let tpl = DashboardInner {
            pending_count: None,
            show_dac_link: false,
            dataset_count: None,
            grant_count: None,
            service_count: None,
            recent_events: vec![],
            events_degraded: true,
            services: vec![super::HealthRow {
                name: "ADS".into(),
                status_class: "health-down".into(),
                status_label: "Down".into(),
                version: "—".into(),
                detail: "service-info unavailable".into(),
            }],
            is_admin: false,
            signing_kid: None,
            signing_algorithm: None,
            signing_rotation_warning: false,
            signing_rotation_due: None,
        };
        let html = tpl.render().expect("render degraded dashboard");
        assert!(html.contains("Audit feed unavailable"));
        assert!(html.contains("Down"));
    }
}
