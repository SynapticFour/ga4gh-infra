// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use ga4gh_types::{AccessRequest, DacActionRequest, DacQueueResponse};
use tracing::instrument;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireDac;
use crate::error::AdsError;
use crate::query::DacGroupQuery;

fn require_reason(reason: &Option<String>, action: &str) -> Result<(), AdsError> {
    if reason.as_ref().is_some_and(|s| !s.trim().is_empty()) {
        Ok(())
    } else {
        Err(AdsError::BadRequest(format!(
            "reason is required for {action}"
        )))
    }
}

fn authorize_dac_action(
    request: &AccessRequest,
    operator_groups: Option<&[String]>,
) -> Result<(), AdsError> {
    let Some(groups) = operator_groups else {
        return Ok(());
    };
    if groups.is_empty() {
        return Err(AdsError::Forbidden);
    }
    match request.dac_group.as_deref() {
        Some(dac) if groups.iter().any(|group| group == dac) => Ok(()),
        _ => Err(AdsError::Forbidden),
    }
}

#[instrument(skip(state))]
pub async fn list_dac_requests(
    State(state): State<Arc<AppState>>,
    RequireDac(_operator): RequireDac,
    Query(filter): Query<DacGroupQuery>,
) -> Result<Json<DacQueueResponse>, AdsError> {
    let requests = state.store.list_dac_requests(filter.filter()).await?;
    Ok(Json(DacQueueResponse { requests }))
}

#[instrument(skip(state, body))]
pub async fn dac_approve(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    let request = state.store.get_access_request(id).await?;
    authorize_dac_action(&request, body.operator_groups.as_deref())?;
    let actor = format!("dac:{}", operator.name);
    let request = state.store.dac_approve(id, &actor, body.reason).await?;
    tracing::info!(
        audit = true,
        event = "dac.approved",
        request_id = %id,
        actor = %actor,
        dac_group = ?request.dac_group,
        "DAC approved access request"
    );
    Ok(Json(request))
}

#[instrument(skip(state, body))]
pub async fn dac_reject(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    require_reason(&body.reason, "reject")?;
    let request = state.store.get_access_request(id).await?;
    authorize_dac_action(&request, body.operator_groups.as_deref())?;
    let actor = format!("dac:{}", operator.name);
    let request = state.store.dac_reject(id, &actor, body.reason).await?;
    tracing::info!(
        audit = true,
        event = "dac.rejected",
        request_id = %id,
        actor = %actor,
        dac_group = ?request.dac_group,
        "DAC rejected access request"
    );
    Ok(Json(request))
}

#[instrument(skip(state, body))]
pub async fn dac_escalate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    RequireDac(operator): RequireDac,
    Json(body): Json<DacActionRequest>,
) -> Result<Json<AccessRequest>, AdsError> {
    require_reason(&body.reason, "escalate")?;
    let request = state.store.get_access_request(id).await?;
    authorize_dac_action(&request, body.operator_groups.as_deref())?;
    let actor = format!("dac:{}", operator.name);
    let request = state.store.dac_escalate(id, &actor, body.reason).await?;
    tracing::info!(
        audit = true,
        event = "dac.escalated",
        request_id = %id,
        actor = %actor,
        dac_group = ?request.dac_group,
        "DAC escalated access request"
    );
    Ok(Json(request))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ga4gh_types::AccessRequestStatus;

    fn request(dac_group: Option<&str>) -> AccessRequest {
        AccessRequest {
            id: Uuid::nil(),
            researcher_id: "r".into(),
            dataset_id: Uuid::nil(),
            project_id: Uuid::nil(),
            justification: None,
            status: AccessRequestStatus::Pending,
            duo_evaluation: None,
            dac_group: dac_group.map(str::to_string),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn unrestricted_api_key_may_approve_any_group() {
        assert!(authorize_dac_action(&request(Some("ega-dac")), None).is_ok());
    }

    #[test]
    fn scoped_operator_may_approve_own_group() {
        let groups = ["ega-dac".to_string()];
        assert!(authorize_dac_action(&request(Some("ega-dac")), Some(&groups)).is_ok());
    }

    #[test]
    fn scoped_operator_cannot_approve_other_group() {
        let groups = ["ega-dac".to_string()];
        assert!(authorize_dac_action(&request(Some("local-dac")), Some(&groups)).is_err());
    }

    #[test]
    fn empty_scope_is_forbidden() {
        let groups: [String; 0] = [];
        assert!(authorize_dac_action(&request(Some("ega-dac")), Some(&groups)).is_err());
    }

    #[test]
    fn staff_group_is_not_implicitly_authorized() {
        let groups = ["staff".to_string()];
        assert!(authorize_dac_action(&request(Some("ega-dac")), Some(&groups)).is_err());
    }
}
