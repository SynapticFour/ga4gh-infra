use std::collections::HashMap;

use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, Response};
use axum::Form;
use ga4gh_types::{
    AccessRequest, AccessRequestStatus, Dataset, DuoEvaluationResult, ResearchProject,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::datetime::FormattedDateTime;
use crate::entity::EntityRef;
use crate::handlers::{is_htmx, render_layout, SharedState};
use crate::roles::operator_dac_groups;
use crate::session::RequireAuth;
use tracing::info;

#[derive(Template)]
#[template(path = "dac/queue_page.html")]
pub struct QueuePageInner {
    pub degraded: bool,
    pub initial_html: String,
}

#[derive(Template)]
#[template(path = "dac/queue_partial.html")]
pub struct QueuePartial {
    pub requests: Vec<QueueRow>,
    pub degraded: bool,
    pub message: Option<String>,
}

#[derive(Template)]
#[template(path = "dac/queue_row.html")]
pub struct QueueRowTemplate {
    pub row: QueueRow,
}

pub struct QueueRow {
    pub id: String,
    pub requester: String,
    pub dataset: EntityRef,
    pub project: EntityRef,
    pub dac_group: String,
    pub status: String,
    pub duo_summary: String,
    pub duo_compatible: bool,
    pub submitted: FormattedDateTime,
    pub resolved: bool,
}

fn status_label(status: AccessRequestStatus) -> &'static str {
    match status {
        AccessRequestStatus::Pending => "Pending",
        AccessRequestStatus::Approved => "Approved",
        AccessRequestStatus::Rejected => "Rejected",
        AccessRequestStatus::Escalated => "Escalated",
    }
}

fn duo_display(eval: Option<&DuoEvaluationResult>) -> (String, bool) {
    match eval {
        Some(e) if e.compatible => (format!("Compatible — score {}/100", e.score), true),
        Some(e) => (format!("Incompatible — score {}/100", e.score), false),
        None => ("Not evaluated".into(), false),
    }
}

fn dac_groups_for(auth: &RequireAuth, state: &SharedState) -> Option<Vec<String>> {
    operator_dac_groups(&auth.0, &state.config.admin_claim_value)
}

async fn lookup_maps(
    state: &SharedState,
    groups: Option<&[String]>,
) -> (HashMap<Uuid, Dataset>, HashMap<Uuid, ResearchProject>) {
    let datasets = state
        .clients
        .ads_list_datasets(groups)
        .await
        .unwrap_or_default();
    let projects = state.clients.ads_list_projects().await.unwrap_or_default();
    let dataset_map = datasets.into_iter().map(|d| (d.id, d)).collect();
    let project_map = projects.into_iter().map(|p| (p.id, p)).collect();
    (dataset_map, project_map)
}

fn build_queue_row(
    request: &AccessRequest,
    datasets: &HashMap<Uuid, Dataset>,
    projects: &HashMap<Uuid, ResearchProject>,
) -> QueueRow {
    let dataset_refs: HashMap<Uuid, &Dataset> = datasets.iter().map(|(k, v)| (*k, v)).collect();
    let project_refs: HashMap<Uuid, &ResearchProject> =
        projects.iter().map(|(k, v)| (*k, v)).collect();
    let (duo_summary, duo_compatible) = duo_display(request.duo_evaluation.as_ref());
    let resolved = !matches!(
        request.status,
        AccessRequestStatus::Pending | AccessRequestStatus::Escalated
    );
    QueueRow {
        id: request.id.to_string(),
        requester: request.researcher_id.clone(),
        dataset: EntityRef::dataset(request.dataset_id, &dataset_refs),
        project: EntityRef::project(request.project_id, &project_refs),
        dac_group: request.dac_group.clone().unwrap_or_else(|| "—".into()),
        status: status_label(request.status).to_string(),
        duo_summary,
        duo_compatible,
        submitted: FormattedDateTime::from_utc(request.created_at),
        resolved,
    }
}

async fn build_queue_rows(
    state: &SharedState,
    groups: Option<&[String]>,
    requests: &[AccessRequest],
) -> Vec<QueueRow> {
    let (datasets, projects) = lookup_maps(state, groups).await;
    requests
        .iter()
        .map(|r| build_queue_row(r, &datasets, &projects))
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct DacActionForm {
    pub reason: Option<String>,
}

async fn render_queue_partial(
    state: &SharedState,
    groups: Option<&[String]>,
    requests: Result<Vec<AccessRequest>, crate::error::AdminUiError>,
) -> Result<String, askama::Error> {
    match requests {
        Ok(requests) => {
            let rows = build_queue_rows(state, groups, &requests).await;
            QueuePartial {
                requests: rows,
                degraded: false,
                message: None,
            }
            .render()
        }
        Err(err) => QueuePartial {
            requests: vec![],
            degraded: true,
            message: Some(err.to_string()),
        }
        .render(),
    }
}

pub async fn queue_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let groups = dac_groups_for(&auth, &state);
    let queue_result = state.clients.ads_dac_queue(groups.as_deref()).await;
    let degraded = queue_result.is_err();
    let initial_html = match render_queue_partial(&state, groups.as_deref(), queue_result).await {
        Ok(html) => html,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    };
    let inner = QueuePageInner {
        degraded,
        initial_html,
    };

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
    match render_queue_partial(
        &state,
        groups.as_deref(),
        state.clients.ads_dac_queue(groups.as_deref()).await,
    )
    .await
    {
        Ok(html) => Html(html).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

async fn dac_action_response(
    result: crate::error::AdminResult<AccessRequest>,
    headers: &HeaderMap,
    state: &SharedState,
    groups: Option<&[String]>,
) -> Response {
    match result {
        Ok(request) => {
            info!(
                request_id = %request.id,
                status = ?request.status,
                "dac action completed"
            );
            if is_htmx(headers) {
                let (datasets, projects) = lookup_maps(state, groups).await;
                return QueueRowTemplate {
                    row: build_queue_row(&request, &datasets, &projects),
                }
                .into_response();
            }
            axum::response::Redirect::to("/dac").into_response()
        }
        Err(err) => {
            tracing::warn!(error = %err, "dac action failed");
            if is_htmx(headers) {
                return (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()).into_response();
            }
            (StatusCode::BAD_REQUEST, err.to_string()).into_response()
        }
    }
}

pub async fn approve(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Form(form): Form<DacActionForm>,
) -> Response {
    let groups = dac_groups_for(&auth, &state);
    let reason = form.reason.filter(|s| !s.trim().is_empty());
    dac_action_response(
        state.clients.ads_approve(id, reason).await,
        &headers,
        &state,
        groups.as_deref(),
    )
    .await
}

pub async fn reject(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Form(form): Form<DacActionForm>,
) -> Response {
    let groups = dac_groups_for(&auth, &state);
    let reason = form.reason.filter(|s| !s.trim().is_empty());
    if reason.is_none() {
        return (StatusCode::BAD_REQUEST, "reason is required for reject").into_response();
    }
    dac_action_response(
        state.clients.ads_reject(id, reason).await,
        &headers,
        &state,
        groups.as_deref(),
    )
    .await
}

pub async fn escalate(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Form(form): Form<DacActionForm>,
) -> Response {
    let groups = dac_groups_for(&auth, &state);
    let reason = form.reason.filter(|s| !s.trim().is_empty());
    if reason.is_none() {
        return (StatusCode::BAD_REQUEST, "reason is required for escalate").into_response();
    }
    dac_action_response(
        state.clients.ads_escalate(id, reason).await,
        &headers,
        &state,
        groups.as_deref(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::QueuePartial;
    use crate::datetime::FormattedDateTime;
    use crate::entity::EntityRef;
    use askama::Template;

    fn sample_row() -> super::QueueRow {
        super::QueueRow {
            id: "req-1".into(),
            requester: "researcher@example.org".into(),
            dataset: EntityRef {
                id: "ds-1".into(),
                name: "Demo Cohort".into(),
                subtitle: Some("dataset-demo".into()),
                href: "/datasets/ds-1".into(),
            },
            project: EntityRef {
                id: "pr-1".into(),
                name: "Pilot Study".into(),
                subtitle: None,
                href: "/projects/pr-1".into(),
            },
            dac_group: "local-dac".into(),
            status: "Pending".into(),
            duo_summary: "Compatible — score 100/100".into(),
            duo_compatible: true,
            submitted: FormattedDateTime {
                display: "2026-06-17 18:55 UTC".into(),
                iso: "2026-06-17T18:55:00+00:00".into(),
            },
            resolved: false,
        }
    }

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

    #[test]
    fn queue_row_shows_dataset_name_not_bare_uuid() {
        let tpl = QueuePartial {
            requests: vec![sample_row()],
            degraded: false,
            message: None,
        };
        let html = tpl.render().expect("render queue");
        assert!(html.contains("Demo Cohort"));
        assert!(html.contains("Pilot Study"));
        assert!(html.contains("2026-06-17 18:55 UTC"));
    }

    #[test]
    fn resolved_queue_row_shows_status_without_actions() {
        let mut row = sample_row();
        row.status = "Approved".into();
        row.resolved = true;
        let tpl = QueuePartial {
            requests: vec![row],
            degraded: false,
            message: None,
        };
        let html = tpl.render().expect("render resolved row");
        assert!(html.contains("Approved"));
        assert!(html.contains("no further action"));
        assert!(html.contains("queue-row-resolved"));
        assert!(!html.contains("btn-primary"));
    }

    #[test]
    fn empty_queue_shows_clear_message() {
        let tpl = QueuePartial {
            requests: vec![],
            degraded: false,
            message: None,
        };
        let html = tpl.render().expect("render empty queue");
        assert!(html.contains("Queue is clear"));
    }
}
