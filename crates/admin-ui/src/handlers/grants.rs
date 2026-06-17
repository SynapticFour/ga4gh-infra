use std::collections::HashMap;

use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, Response};
use ga4gh_types::{Dataset, Grant};
use uuid::Uuid;

use crate::csv;
use crate::datetime::FormattedDateTime;
use crate::entity::EntityRef;
use crate::handlers::{htmx_redirect, is_htmx, render_layout, SharedState};
use crate::roles::operator_dac_groups;
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "grants/list.html")]
struct ListInner {
    pub grants: Vec<GrantRow>,
    pub degraded: bool,
    pub is_admin: bool,
}

pub struct GrantRow {
    pub id: String,
    pub researcher_id: String,
    pub dataset: EntityRef,
    pub source: String,
    pub source_class: String,
    pub status: String,
    pub issued: FormattedDateTime,
    pub expires: FormattedDateTime,
}

fn build_grant_row(g: &Grant, datasets: &HashMap<Uuid, &Dataset>) -> GrantRow {
    let (source, source_class) = match g.source {
        ga4gh_types::GrantSource::DacApproval => ("DAC approval".into(), "badge-dac".into()),
        ga4gh_types::GrantSource::DuoAutoApproval => ("DUO auto".into(), "badge-duo".into()),
        ga4gh_types::GrantSource::InstitutionalMapping => {
            ("Institutional".into(), "badge-inst".into())
        }
    };
    GrantRow {
        id: g.id.to_string(),
        researcher_id: g.researcher_id.clone(),
        dataset: EntityRef::dataset(g.dataset_id, datasets),
        source,
        source_class,
        status: if g.revoked_at.is_some() {
            "Revoked".into()
        } else {
            "Active".into()
        },
        issued: FormattedDateTime::from_utc(g.created_at),
        expires: FormattedDateTime::optional(g.expires_at),
    }
}

async fn load_grants(
    auth: &RequireAuth,
    state: &SharedState,
) -> Result<Vec<Grant>, crate::error::AdminUiError> {
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    if auth.0.is_admin {
        state.clients.ads_list_grants(None, None).await
    } else {
        state
            .clients
            .ads_list_grants_for_operator(&auth.0.sub, groups.as_deref())
            .await
    }
}

async fn load_grant_rows(
    auth: &RequireAuth,
    state: &SharedState,
) -> Result<Vec<GrantRow>, crate::error::AdminUiError> {
    let grants = load_grants(auth, state).await?;
    let groups = operator_dac_groups(&auth.0, &state.config.admin_claim_value);
    let dataset_groups = if auth.0.is_admin {
        None
    } else {
        groups.as_deref()
    };
    let datasets = state
        .clients
        .ads_list_datasets(dataset_groups)
        .await
        .unwrap_or_default();
    let dataset_map: HashMap<Uuid, Dataset> = datasets.into_iter().map(|d| (d.id, d)).collect();
    let dataset_refs: HashMap<Uuid, &Dataset> = dataset_map.iter().map(|(k, v)| (*k, v)).collect();
    Ok(grants
        .iter()
        .map(|g| build_grant_row(g, &dataset_refs))
        .collect())
}

pub async fn list_page(auth: RequireAuth, State(state): State<SharedState>) -> impl IntoResponse {
    let result = load_grant_rows(&auth, &state).await;
    let degraded = result.is_err();
    let inner = ListInner {
        grants: result.unwrap_or_default(),
        degraded,
        is_admin: auth.0.is_admin,
    };
    match render_layout("Grants", "grants", &auth.0, inner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn export_csv(auth: RequireAuth, State(state): State<SharedState>) -> Response {
    let grants = match load_grant_rows(&auth, &state).await {
        Ok(grants) => grants,
        Err(err) => return (StatusCode::SERVICE_UNAVAILABLE, err.to_string()).into_response(),
    };

    let mut body = csv::row(&[
        "id",
        "researcher_id",
        "dataset_id",
        "dataset_name",
        "source",
        "status",
        "issued",
        "expires",
    ]);
    for row in &grants {
        body.push_str(&csv::row(&[
            &row.id,
            &row.researcher_id,
            &row.dataset.id,
            &row.dataset.name,
            &row.source,
            &row.status,
            &row.issued.iso,
            if row.expires.has_iso() {
                &row.expires.iso
            } else {
                ""
            },
        ]));
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"grants.csv\"",
            ),
        ],
        body,
    )
        .into_response()
}

pub async fn revoke(
    auth: RequireAuth,
    State(state): State<SharedState>,
    Path(id): Path<uuid::Uuid>,
    headers: HeaderMap,
) -> Response {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err();
    }
    match state.clients.ads_revoke_grant(id).await {
        Ok(()) => {
            if is_htmx(&headers) {
                let mut h = HeaderMap::new();
                htmx_redirect(&mut h, "/grants");
                (h, StatusCode::NO_CONTENT).into_response()
            } else {
                axum::response::Redirect::to("/grants").into_response()
            }
        }
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
    }
}
