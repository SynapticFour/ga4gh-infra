// SPDX-License-Identifier: Apache-2.0

//! PostgreSQL and SQLite persistence for ADS entities.
//!
//! DAC authorization is implemented in `handlers/dac.rs`, not here.

pub(crate) use std::collections::{BTreeMap, HashMap, HashSet};
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;

pub(crate) use crate::config::{DatabaseConfig, DatabaseDriver};
pub(crate) use crate::error::AdsError;
pub(crate) use crate::events::{
    grant_created, grant_revoked, request_approved, request_created, request_rejected,
};
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use ga4gh_types::{
    AccessDecision, AccessDecisionOutcome, AccessRequest, AccessRequestStatus, AdsEvent,
    AdsEventType, AdsResourceType, CreateAccessRequestBody, CreateDatasetRequest,
    CreatePermissionMappingRequest, CreatePermissionSourceRequest, CreateProjectRequest,
    CreateVisaSourceRequest, Dataset, DatasetVisibility, DuoCode, DuoEvaluationResult, Grant,
    GrantSource, PermissionMapping, PermissionSource, ResearchProject, Researcher, VisaSource,
};
#[cfg(feature = "postgres")]
pub(crate) use sqlx::PgPool;
pub(crate) use sqlx::Row;
#[cfg(feature = "sqlite")]
pub(crate) use sqlx::SqlitePool;
#[allow(unused_imports)]
pub(crate) use tracing::instrument;
pub(crate) use uuid::Uuid;

pub(crate) fn empty_dac_filter(dac_groups: Option<&[String]>) -> bool {
    matches!(dac_groups, Some(groups) if groups.is_empty())
}

pub(crate) fn webhook_http_client() -> Result<reqwest::Client, AdsError> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|err| AdsError::Internal(err.to_string()))
}

pub(crate) fn parse_visibility(raw: &str) -> DatasetVisibility {
    match raw {
        "public" => DatasetVisibility::Public,
        "draft" => DatasetVisibility::Draft,
        _ => DatasetVisibility::Institute,
    }
}

pub(crate) fn visibility_str(v: DatasetVisibility) -> &'static str {
    match v {
        DatasetVisibility::Public => "public",
        DatasetVisibility::Draft => "draft",
        DatasetVisibility::Institute => "institute",
    }
}

pub(crate) fn parse_resource_type(raw: &str) -> AdsResourceType {
    match raw {
        "compute_pool" => AdsResourceType::ComputePool,
        _ => AdsResourceType::Dataset,
    }
}

pub(crate) fn resource_type_str(t: AdsResourceType) -> &'static str {
    match t {
        AdsResourceType::ComputePool => "compute_pool",
        AdsResourceType::Dataset => "dataset",
    }
}

#[derive(Clone)]
enum DbPool {
    #[cfg(feature = "postgres")]
    Postgres(PgPool),
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
}

/// Database-backed ADS store.
#[derive(Clone)]
pub struct AdsStore {
    pool: DbPool,
    webhook_urls: Arc<Vec<String>>,
    http: reqwest::Client,
    api_key_pepper: String,
}

macro_rules! with_pool {
    ($self:expr, $pool:ident, $body:expr) => {{
        match &$self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres($pool) => $body,
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite($pool) => $body,
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }};
}

/// Joined permission mapping row used for institutional grant evaluation.
pub(crate) struct ActivePermissionMapping {
    claim_path: String,
    claim_value: String,
    dataset_id: Uuid,
    grant_lifetime_seconds: Option<u64>,
}

pub(crate) fn unix_now() -> i64 {
    Utc::now().timestamp()
}

pub(crate) fn dt_from_ts(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

pub(crate) fn map_db_err(err: impl ToString) -> AdsError {
    AdsError::Database(err.to_string())
}

macro_rules! parse_researcher {
    ($row:expr) => {{
        let row = $row;
        let affiliations_json: String = row.try_get("affiliations").map_err(map_db_err)?;
        let affiliations: Vec<ga4gh_types::ResearcherAffiliation> =
            serde_json::from_str(&affiliations_json).map_err(map_db_err)?;
        Researcher {
            id: row.try_get("id").map_err(map_db_err)?,
            display_name: row.try_get("display_name").map_err(map_db_err)?,
            email: row.try_get("email").map_err(map_db_err)?,
            affiliations,
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_dataset {
    ($row:expr) => {{
        let row = $row;
        let duo_json: String = row.try_get("duo_codes").map_err(map_db_err)?;
        let duo_codes: Vec<DuoCode> = serde_json::from_str(&duo_json).map_err(map_db_err)?;
        Dataset {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            name: row.try_get("name").map_err(map_db_err)?,
            description: row.try_get("description").map_err(map_db_err)?,
            duo_codes,
            external_id: row.try_get("external_id").map_err(map_db_err)?,
            auto_approve_enabled: row
                .try_get::<i32, _>("auto_approve_enabled")
                .map_err(map_db_err)?
                != 0,
            auto_approve_threshold: row
                .try_get::<i32, _>("auto_approve_threshold")
                .map_err(map_db_err)? as u8,
            dac_group: row.try_get("dac_group").map_err(map_db_err)?,
            visibility: parse_visibility(
                &row.try_get::<String, _>("visibility")
                    .unwrap_or_else(|_| "institute".into()),
            ),
            resource_type: parse_resource_type(
                &row.try_get::<String, _>("resource_type")
                    .unwrap_or_else(|_| "dataset".into()),
            ),
            remote_drs_base_url: row.try_get("remote_drs_base_url").ok(),
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_project {
    ($row:expr) => {{
        let row = $row;
        let duo_json: String = row.try_get("duo_codes").map_err(map_db_err)?;
        let duo_codes: Vec<DuoCode> = serde_json::from_str(&duo_json).map_err(map_db_err)?;
        ResearchProject {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            researcher_id: row.try_get("researcher_id").map_err(map_db_err)?,
            name: row.try_get("name").map_err(map_db_err)?,
            description: row.try_get("description").map_err(map_db_err)?,
            duo_codes,
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_access_request {
    ($row:expr) => {{
        let row = $row;
        let eval_json: Option<String> = row.try_get("duo_evaluation").map_err(map_db_err)?;
        let duo_evaluation = eval_json
            .map(|json| serde_json::from_str::<DuoEvaluationResult>(&json))
            .transpose()
            .map_err(map_db_err)?;
        AccessRequest {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            researcher_id: row.try_get("researcher_id").map_err(map_db_err)?,
            dataset_id: Uuid::parse_str(
                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
            )
            .map_err(map_db_err)?,
            project_id: Uuid::parse_str(
                &row.try_get::<String, _>("project_id").map_err(map_db_err)?,
            )
            .map_err(map_db_err)?,
            status: parse_status(&row.try_get::<String, _>("status").map_err(map_db_err)?)?,
            justification: row.try_get("justification").map_err(map_db_err)?,
            duo_evaluation,
            dac_group: row.try_get("dac_group").map_err(map_db_err)?,
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
            updated_at: dt_from_ts(row.try_get("updated_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_grant {
    ($row:expr) => {{
        let row = $row;
        let duo_json: String = row.try_get("duo_codes").map_err(map_db_err)?;
        let duo_codes: Vec<DuoCode> = serde_json::from_str(&duo_json).map_err(map_db_err)?;
        let request_id: Option<String> = row.try_get("request_id").map_err(map_db_err)?;
        Grant {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            researcher_id: row.try_get("researcher_id").map_err(map_db_err)?,
            dataset_id: Uuid::parse_str(
                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
            )
            .map_err(map_db_err)?,
            request_id: request_id
                .map(|id| Uuid::parse_str(&id))
                .transpose()
                .map_err(map_db_err)?,
            source: parse_grant_source(&row.try_get::<String, _>("source").map_err(map_db_err)?)?,
            duo_codes,
            resource_scope: row.try_get("resource_scope").map_err(map_db_err)?,
            expires_at: row
                .try_get::<Option<i64>, _>("expires_at")
                .map_err(map_db_err)?
                .map(dt_from_ts),
            revoked_at: row
                .try_get::<Option<i64>, _>("revoked_at")
                .map_err(map_db_err)?
                .map(dt_from_ts),
            created_at: dt_from_ts(row.try_get("created_at").map_err(map_db_err)?),
        }
    }};
}

macro_rules! parse_audit_event {
    ($row:expr) => {{
        let payload_raw: String = $row.try_get("payload").map_err(map_db_err)?;
        let payload: BTreeMap<String, serde_json::Value> =
            serde_json::from_str(&payload_raw).map_err(map_db_err)?;
        AdsEvent {
            id: Uuid::parse_str(&$row.try_get::<String, _>("id").map_err(map_db_err)?)
                .map_err(map_db_err)?,
            event_type: parse_event_type(
                &$row
                    .try_get::<String, _>("event_type")
                    .map_err(map_db_err)?,
            )?,
            occurred_at: dt_from_ts($row.try_get("occurred_at").map_err(map_db_err)?),
            payload,
        }
    }};
}

#[allow(unused_imports)]
pub(crate) use parse_access_request;
#[allow(unused_imports)]
pub(crate) use parse_audit_event;
#[allow(unused_imports)]
pub(crate) use parse_dataset;
#[allow(unused_imports)]
pub(crate) use parse_grant;
#[allow(unused_imports)]
pub(crate) use parse_project;
#[allow(unused_imports)]
pub(crate) use parse_researcher;
#[allow(unused_imports)]
pub(crate) use with_pool;

mod connect;
mod datasets;
mod events;
mod grants;
mod permissions;
mod projects;
mod requests;
mod researchers;

pub(crate) fn status_str(status: AccessRequestStatus) -> &'static str {
    match status {
        AccessRequestStatus::Pending => "pending",
        AccessRequestStatus::Approved => "approved",
        AccessRequestStatus::Rejected => "rejected",
        AccessRequestStatus::Escalated => "escalated",
        _ => "unknown",
    }
}

pub(crate) fn parse_status(raw: &str) -> Result<AccessRequestStatus, AdsError> {
    match raw {
        "pending" => Ok(AccessRequestStatus::Pending),
        "approved" => Ok(AccessRequestStatus::Approved),
        "rejected" => Ok(AccessRequestStatus::Rejected),
        "escalated" => Ok(AccessRequestStatus::Escalated),
        other => Err(AdsError::Internal(format!("unknown status {other}"))),
    }
}

pub(crate) fn outcome_str(outcome: AccessDecisionOutcome) -> &'static str {
    match outcome {
        AccessDecisionOutcome::Approved => "approved",
        AccessDecisionOutcome::Rejected => "rejected",
        AccessDecisionOutcome::Escalated => "escalated",
    }
}

pub(crate) fn grant_source_str(source: &GrantSource) -> &'static str {
    match source {
        GrantSource::DacApproval => "dac_approval",
        GrantSource::DuoAutoApproval => "duo_auto_approval",
        GrantSource::InstitutionalMapping => "institutional_mapping",
    }
}

pub(crate) fn parse_grant_source(raw: &str) -> Result<GrantSource, AdsError> {
    match raw {
        "dac_approval" => Ok(GrantSource::DacApproval),
        "duo_auto_approval" => Ok(GrantSource::DuoAutoApproval),
        "institutional_mapping" => Ok(GrantSource::InstitutionalMapping),
        other => Err(AdsError::Internal(format!("unknown grant source {other}"))),
    }
}

pub(crate) fn event_type_str(event_type: &AdsEventType) -> &'static str {
    match event_type {
        AdsEventType::GrantCreated => "grant.created",
        AdsEventType::GrantRevoked => "grant.revoked",
        AdsEventType::RequestCreated => "request.created",
        AdsEventType::RequestApproved => "request.approved",
        AdsEventType::RequestRejected => "request.rejected",
        _ => "unknown",
    }
}

pub(crate) fn parse_event_type(raw: &str) -> Result<AdsEventType, AdsError> {
    match raw {
        "grant.created" => Ok(AdsEventType::GrantCreated),
        "grant.revoked" => Ok(AdsEventType::GrantRevoked),
        "request.created" => Ok(AdsEventType::RequestCreated),
        "request.approved" => Ok(AdsEventType::RequestApproved),
        "request.rejected" => Ok(AdsEventType::RequestRejected),
        other => Err(AdsError::Internal(format!("unknown event type {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ga4gh_types::DuoCode;

    async fn test_store() -> AdsStore {
        AdsStore::connect(
            &DatabaseConfig {
                driver: DatabaseDriver::Sqlite,
                url: Some("sqlite::memory:".to_string()),
                url_env: "ADS_DATABASE_URL".to_string(),
                auto_migrate: true,
            },
            "sqlite::memory:",
            vec![],
        )
        .await
        .expect("memory store")
    }

    #[tokio::test]
    async fn dataset_and_grant_lifecycle() {
        let store = test_store().await;
        let dataset = store
            .create_dataset(&CreateDatasetRequest {
                name: "Test dataset".to_string(),
                description: None,
                duo_codes: vec![DuoCode::Gru],
                external_id: Some("drs:abc".to_string()),
                auto_approve_enabled: true,
                auto_approve_threshold: 100,
                dac_group: None,
                visibility: DatasetVisibility::Institute,
                resource_type: AdsResourceType::Dataset,
                remote_drs_base_url: None,
            })
            .await
            .expect("create dataset");

        let project = store
            .create_project(&CreateProjectRequest {
                researcher_id: "researcher@example.org".to_string(),
                name: "Project".to_string(),
                description: None,
                duo_codes: vec![DuoCode::Gru],
            })
            .await
            .expect("create project");

        let evaluation = crate::duo::evaluate_duo_codes(
            &dataset.duo_codes,
            &project.duo_codes,
            dataset.auto_approve_threshold,
        );
        let request = store
            .create_access_request(
                &CreateAccessRequestBody {
                    researcher_id: "researcher@example.org".to_string(),
                    dataset_id: dataset.id,
                    project_id: project.id,
                    justification: None,
                },
                Some(evaluation),
            )
            .await
            .expect("create request");

        assert_eq!(request.status, AccessRequestStatus::Approved);
        let grants = store
            .list_grants(Some("researcher@example.org"), None)
            .await
            .expect("list grants");
        assert_eq!(grants.len(), 1);
    }
}
