// SPDX-License-Identifier: Apache-2.0

//! GA4GH Access Decision Service (ADS) domain types.
//!
//! ADS bridges GA4GH AAI Passports and resource services (Beacon, DRS, htsget, WES, TES)
//! by managing access requests, DUO evaluation, DAC workflows, grants, and visa export.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::duo::DuoCode;
use crate::visa::{VisaClaim, VisaType};

/// Canonical researcher identity (OIDC `sub` from the GA4GH AAI broker).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Researcher {
    /// OIDC subject — canonical researcher identifier.
    pub id: String,
    /// Optional display name from upstream IdP claims.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Optional email from upstream IdP claims.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Institutional affiliations used for AffiliationAndRole visa export.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affiliations: Vec<ResearcherAffiliation>,
    /// When the researcher record was first observed.
    pub created_at: DateTime<Utc>,
    /// When the researcher record was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Institutional affiliation for visa assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearcherAffiliation {
    /// Organization identifier (e.g. ROR or email-domain style value).
    pub organization: String,
    /// Role within the organization (e.g. `faculty`, `staff`).
    pub role: String,
}

/// A controlled-access dataset registered with ADS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dataset {
    /// Stable dataset identifier.
    pub id: Uuid,
    /// Human-readable dataset name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// DUO codes attached to the dataset policy.
    pub duo_codes: Vec<DuoCode>,
    /// Optional external resource identifier (DRS, Beacon dataset id, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// Whether DUO-compatible requests may be auto-approved without DAC review.
    #[serde(default)]
    pub auto_approve_enabled: bool,
    /// Minimum compatibility score (0–100) required for auto-approval.
    #[serde(default = "default_auto_approve_threshold")]
    pub auto_approve_threshold: u8,
    /// When the dataset was registered.
    pub created_at: DateTime<Utc>,
    /// When the dataset was last updated.
    pub updated_at: DateTime<Utc>,
}

fn default_auto_approve_threshold() -> u8 {
    100
}

/// Request body for registering a dataset.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateDatasetRequest {
    /// Human-readable dataset name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// DUO codes attached to the dataset policy.
    pub duo_codes: Vec<DuoCode>,
    /// Optional external resource identifier.
    pub external_id: Option<String>,
    /// Whether DUO-compatible requests may be auto-approved.
    #[serde(default)]
    pub auto_approve_enabled: bool,
    /// Minimum compatibility score for auto-approval (0–100).
    #[serde(default = "default_auto_approve_threshold")]
    pub auto_approve_threshold: u8,
}

/// A research project with intended-use DUO annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchProject {
    /// Stable project identifier.
    pub id: Uuid,
    /// Researcher OIDC subject owning the project.
    pub researcher_id: String,
    /// Project title.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// DUO codes describing intended use.
    pub duo_codes: Vec<DuoCode>,
    /// When the project was registered.
    pub created_at: DateTime<Utc>,
    /// When the project was last updated.
    pub updated_at: DateTime<Utc>,
}
/// Request body for registering a research project.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateProjectRequest {
    /// Researcher OIDC subject (must match authenticated caller unless admin).
    pub researcher_id: String,
    /// Project title.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// DUO codes describing intended use.
    pub duo_codes: Vec<DuoCode>,
}

/// Lifecycle status of an access request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessRequestStatus {
    /// Awaiting DAC review or auto-approval evaluation.
    Pending,
    /// Approved by DAC or auto-approval.
    Approved,
    /// Rejected by DAC.
    Rejected,
    /// Escalated to a higher review tier.
    Escalated,
}

/// A researcher request for access to a dataset under a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessRequest {
    /// Stable request identifier.
    pub id: Uuid,
    /// Researcher OIDC subject.
    pub researcher_id: String,
    /// Target dataset.
    pub dataset_id: Uuid,
    /// Research project providing intended-use DUO context.
    pub project_id: Uuid,
    /// Current workflow status.
    pub status: AccessRequestStatus,
    /// Optional researcher justification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    /// DUO evaluation snapshot at submission time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duo_evaluation: Option<DuoEvaluationResult>,
    /// When the request was submitted.
    pub created_at: DateTime<Utc>,
    /// When the request was last updated.
    pub updated_at: DateTime<Utc>,
}
/// Request body for submitting an access request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateAccessRequestBody {
    /// Researcher OIDC subject.
    pub researcher_id: String,
    /// Target dataset identifier.
    pub dataset_id: Uuid,
    /// Research project identifier.
    pub project_id: Uuid,
    /// Optional justification for DAC review.
    pub justification: Option<String>,
}

/// Immutable DAC or system decision on an access request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessDecision {
    /// Stable decision record identifier.
    pub id: Uuid,
    /// Parent access request.
    pub request_id: Uuid,
    /// Decision outcome.
    pub outcome: AccessDecisionOutcome,
    /// Actor (`dac:{name}`, `system:duo-auto`, `system:institutional`).
    pub actor: String,
    /// Optional rationale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When the decision was recorded (immutable).
    pub decided_at: DateTime<Utc>,
}

/// Outcome of an access decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessDecisionOutcome {
    /// Access approved.
    Approved,
    /// Access rejected.
    Rejected,
    /// Escalated for further review.
    Escalated,
}

/// Body for DAC approve/reject/escalate actions.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DacActionRequest {
    /// Optional rationale recorded in the audit trail.
    pub reason: Option<String>,
    /// DAC member identifier (defaults to API key name when omitted).
    pub actor: Option<String>,
}

/// Origin of an authorization grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantSource {
    /// Human DAC approval of an access request.
    DacApproval,
    /// Automatic approval after DUO compatibility check.
    DuoAutoApproval,
    /// Institutional OIDC claim or group mapping.
    InstitutionalMapping,
}

/// Canonical permission issued by ADS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// Stable grant identifier.
    pub id: Uuid,
    /// Researcher OIDC subject.
    pub researcher_id: String,
    /// Dataset the grant applies to.
    pub dataset_id: Uuid,
    /// Optional linked access request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,
    /// How the grant was created.
    pub source: GrantSource,
    /// DUO codes effective at grant time.
    pub duo_codes: Vec<DuoCode>,
    /// Optional resource scope (DRS id, Beacon dataset, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_scope: Option<String>,
    /// When the grant becomes inactive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// When the grant was revoked, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    /// When the grant was issued.
    pub created_at: DateTime<Utc>,
}
/// External visa issuer configuration for passport assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaSource {
    /// Stable visa source identifier.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// Organization URL used as visa `source`.
    pub issuer_url: String,
    /// Visa type this source emits.
    pub visa_type: VisaType,
    /// Whether ADS should publish visas from this source to the AAI.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// When the visa source was registered.
    pub created_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

/// Request to register a visa source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateVisaSourceRequest {
    /// Human-readable name.
    pub name: String,
    /// Organization URL used as visa `source`.
    pub issuer_url: String,
    /// Visa type this source emits.
    pub visa_type: VisaType,
    /// Whether ADS should publish visas from this source.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Institutional OIDC claim source for permission mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionSource {
    /// Stable permission source identifier.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// Trusted OIDC issuer URL.
    pub oidc_issuer: String,
    /// JWT claim path (e.g. `groups`, `realm_access.roles`).
    pub claim_path: String,
    /// When the permission source was registered.
    pub created_at: DateTime<Utc>,
}

/// Maps an institutional claim value to dataset access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionMapping {
    /// Stable mapping identifier.
    pub id: Uuid,
    /// Parent permission source identifier.
    pub source_id: Uuid,
    /// Claim value that triggers the mapping (e.g. `ega-approved-researchers`).
    pub claim_value: String,
    /// Dataset granted when the claim matches.
    pub dataset_id: Uuid,
    /// Optional grant lifetime in seconds from mapping application.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_lifetime_seconds: Option<u64>,
    /// When the mapping was registered.
    pub created_at: DateTime<Utc>,
}

/// Request to register a permission source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreatePermissionSourceRequest {
    /// Human-readable name.
    pub name: String,
    /// Trusted OIDC issuer URL.
    pub oidc_issuer: String,
    /// JWT claim path (e.g. `groups`).
    pub claim_path: String,
}

/// Request to register a permission mapping.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreatePermissionMappingRequest {
    /// Parent permission source identifier.
    pub source_id: Uuid,
    /// Claim value that triggers the mapping.
    pub claim_value: String,
    /// Dataset granted when the claim matches.
    pub dataset_id: Uuid,
    /// Optional grant lifetime in seconds.
    pub grant_lifetime_seconds: Option<u64>,
}

/// DUO compatibility evaluation request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DuoEvaluateRequest {
    /// Dataset DUO codes (or dataset id to resolve).
    #[serde(default)]
    pub dataset_duo: Vec<DuoCode>,
    /// Optional dataset id — codes loaded from registry when set.
    pub dataset_id: Option<Uuid>,
    /// Project DUO codes (or project id to resolve).
    #[serde(default)]
    pub project_duo: Vec<DuoCode>,
    /// Optional project id.
    pub project_id: Option<Uuid>,
    /// Optional auto-approval threshold override (0–100).
    pub auto_approve_threshold: Option<u8>,
}

/// Result of DUO compatibility evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuoEvaluationResult {
    /// Whether dataset and project DUO policies are compatible.
    pub compatible: bool,
    /// Compatibility score (0–100).
    pub score: u8,
    /// Whether the score meets the auto-approval threshold.
    pub auto_approvable: bool,
    /// Human-readable explanation.
    pub reason: String,
    /// Dataset codes satisfied by the project.
    #[serde(default)]
    pub matched_codes: Vec<DuoCode>,
    /// Dataset codes not satisfied.
    #[serde(default)]
    pub missing_codes: Vec<DuoCode>,
    /// Modifiers requiring DAC procedural review even when compatible.
    #[serde(default)]
    pub procedural_modifiers: Vec<String>,
}

/// Token introspection request from resource services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrospectRequest {
    /// Bearer token (Passport JWT or opaque reference).
    pub token: String,
    /// Resource being accessed (DRS id, Beacon dataset, etc.).
    pub resource: String,
    /// Optional action (`read`, `execute`, etc.).
    pub action: Option<String>,
    /// Optional dataset id for grant lookup.
    pub dataset_id: Option<Uuid>,
}

/// Token introspection response (OAuth2-inspired, ADS-extended).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrospectResponse {
    /// Whether the token is active and access is permitted.
    pub active: bool,
    /// Researcher OIDC subject when active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    /// Matching grant ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grant_ids: Vec<Uuid>,
    /// DUO codes satisfied for the resource.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duo_codes: Vec<DuoCode>,
    /// Token expiry (Unix seconds) when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    /// Explanation when inactive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Visa export payload for AAI passport assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearcherVisasResponse {
    /// Researcher OIDC subject.
    pub researcher_id: String,
    /// Unsigned visa claims ADS would sign or publish via visa-registry.
    pub visas: Vec<VisaClaim>,
}

/// ADS audit/event types emitted to the event log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdsEventType {
    /// A grant was created.
    GrantCreated,
    /// A grant was revoked.
    GrantRevoked,
    /// An access request was submitted.
    RequestCreated,
    /// An access request was approved.
    RequestApproved,
    /// An access request was rejected.
    RequestRejected,
}

/// Immutable audit event record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdsEvent {
    /// Event identifier.
    pub id: Uuid,
    /// Event type.
    pub event_type: AdsEventType,
    /// ISO-8601 timestamp.
    pub occurred_at: DateTime<Utc>,
    /// Structured event payload.
    pub payload: BTreeMap<String, serde_json::Value>,
}

/// Paginated grant list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantListResponse {
    /// Active or listed grants.
    pub grants: Vec<Grant>,
}

/// Paginated DAC queue response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DacQueueResponse {
    /// Access requests awaiting DAC review.
    pub requests: Vec<AccessRequest>,
}

/// Broker/service sync of researcher identity and upstream OIDC claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearcherSyncRequest {
    /// OIDC subject.
    pub sub: String,
    /// Display name from upstream IdP.
    pub display_name: Option<String>,
    /// Email from upstream IdP.
    pub email: Option<String>,
    /// Full upstream claim set for institutional permission mapping.
    #[serde(default)]
    pub claims: BTreeMap<String, serde_json::Value>,
    /// Affiliations for AffiliationAndRole visa export.
    #[serde(default)]
    pub affiliations: Vec<ResearcherAffiliation>,
}

/// Signed visa JWTs ready for passport assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedVisasResponse {
    /// Researcher OIDC subject.
    pub researcher_id: String,
    /// Signed visa JWT strings.
    pub visa_jwts: Vec<String>,
}
