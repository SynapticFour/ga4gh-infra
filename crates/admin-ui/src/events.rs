//! Human-readable labels for ADS audit events.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ga4gh_types::{AdsEvent, AdsEventType, Dataset, Grant, ResearchProject};
use uuid::Uuid;

/// Names and lookup tables used to turn UUIDs in event payloads into friendly labels.
#[derive(Debug, Clone, Default)]
pub struct EventLabelContext {
    datasets: HashMap<Uuid, String>,
    projects: HashMap<Uuid, String>,
    grants: HashMap<Uuid, (String, Uuid)>,
}

impl EventLabelContext {
    pub fn from_catalog(
        datasets: &[Dataset],
        projects: &[ResearchProject],
        grants: &[Grant],
    ) -> Self {
        Self {
            datasets: datasets
                .iter()
                .map(|d| (d.id, dataset_display(d)))
                .collect(),
            projects: projects.iter().map(|p| (p.id, p.name.clone())).collect(),
            grants: grants
                .iter()
                .map(|g| (g.id, (g.researcher_id.clone(), g.dataset_id)))
                .collect(),
        }
    }

    fn dataset_label(&self, id: &str) -> String {
        parse_uuid(id)
            .and_then(|id| self.datasets.get(&id).cloned())
            .unwrap_or_else(|| short_id(id))
    }

    fn project_label(&self, id: &str) -> String {
        parse_uuid(id)
            .and_then(|id| self.projects.get(&id).cloned())
            .unwrap_or_else(|| short_id(id))
    }

    fn grant_context(&self, grant_id: &str) -> Option<(String, String)> {
        let id = parse_uuid(grant_id)?;
        if let Some((researcher, dataset_id)) = self.grants.get(&id) {
            let dataset = self
                .datasets
                .get(dataset_id)
                .cloned()
                .unwrap_or_else(|| short_id(&dataset_id.to_string()));
            return Some((researcher.clone(), dataset));
        }
        None
    }
}

/// Format an audit event as a short, human-readable sentence for the dashboard feed.
pub fn format_event_label(event: &AdsEvent, ctx: &EventLabelContext) -> String {
    let researcher = payload_str(event, "researcher_id");
    let dataset = payload_str(event, "dataset_id");
    let project = payload_str(event, "project_id");
    let request = payload_str(event, "request_id");
    let grant = payload_str(event, "grant_id");
    let actor = payload_str(event, "actor").unwrap_or_else(|| "system".to_string());

    match event.event_type {
        AdsEventType::GrantCreated => {
            if let (Some(r), Some(d)) = (researcher, dataset) {
                format!("Grant issued to {r} for {}", ctx.dataset_label(&d))
            } else if let Some(g) = grant {
                if let Some((r, d)) = ctx.grant_context(&g) {
                    format!("Grant issued to {r} for {d}")
                } else {
                    format!("Grant {} created", short_id(&g))
                }
            } else {
                "Grant created".to_string()
            }
        }
        AdsEventType::GrantRevoked => {
            if let (Some(r), Some(d)) = (researcher, dataset) {
                format!("Grant for {} ({r}) revoked", ctx.dataset_label(&d))
            } else if let Some(g) = grant {
                if let Some((r, d)) = ctx.grant_context(&g) {
                    format!("Grant for {d} ({r}) revoked")
                } else {
                    format!("Grant {} revoked", short_id(&g))
                }
            } else {
                "Grant revoked".to_string()
            }
        }
        AdsEventType::RequestCreated => {
            if let (Some(r), Some(d)) = (researcher, dataset) {
                let dataset_name = ctx.dataset_label(&d);
                if let Some(p) = project {
                    format!(
                        "{r} submitted an access request for {dataset_name} (project {})",
                        ctx.project_label(&p)
                    )
                } else {
                    format!("{r} submitted an access request for {dataset_name}")
                }
            } else if let Some(req) = request {
                format!("Access request {} submitted", short_id(&req))
            } else {
                "Access request submitted".to_string()
            }
        }
        AdsEventType::RequestApproved => {
            if let Some(d) = dataset {
                let dataset_name = ctx.dataset_label(&d);
                if let Some(r) = researcher {
                    format!("Access to {dataset_name} approved for {r} by {actor}")
                } else {
                    format!("Access to {dataset_name} approved by {actor}")
                }
            } else if let Some(req) = request {
                format!("Request {} approved by {actor}", short_id(&req))
            } else {
                format!("Access request approved by {actor}")
            }
        }
        AdsEventType::RequestRejected => {
            if let Some(d) = dataset {
                let dataset_name = ctx.dataset_label(&d);
                if let Some(r) = researcher {
                    format!("Access to {dataset_name} denied for {r} by {actor}")
                } else {
                    format!("Access to {dataset_name} denied by {actor}")
                }
            } else if let Some(req) = request {
                format!("Request {} rejected by {actor}", short_id(&req))
            } else {
                format!("Access request rejected by {actor}")
            }
        }
    }
}

/// Relative time phrase such as "2 min ago" for recent activity.
pub fn format_relative_time(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (now - at).num_seconds().max(0);
    if secs < 60 {
        return if secs <= 1 {
            "just now".to_string()
        } else {
            format!("{secs} sec ago")
        };
    }
    let mins = secs / 60;
    if mins < 60 {
        return if mins == 1 {
            "1 min ago".to_string()
        } else {
            format!("{mins} min ago")
        };
    }
    let hours = mins / 60;
    if hours < 48 {
        return if hours == 1 {
            "1 hr ago".to_string()
        } else {
            format!("{hours} hr ago")
        };
    }
    let days = hours / 24;
    if days == 1 {
        "1 day ago".to_string()
    } else {
        format!("{days} days ago")
    }
}

fn dataset_display(d: &Dataset) -> String {
    if let Some(ext) = d.external_id.as_ref().filter(|s| !s.is_empty()) {
        format!("{} ({ext})", d.name)
    } else {
        d.name.clone()
    }
}

fn parse_uuid(s: &str) -> Option<Uuid> {
    Uuid::parse_str(s.trim()).ok()
}

fn short_id(id: &str) -> String {
    let s = id.trim();
    if s.len() > 8 {
        format!("{}…", &s[..8])
    } else {
        s.to_string()
    }
}

fn payload_str(event: &AdsEvent, key: &str) -> Option<String> {
    event.payload.get(key).map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string().trim_matches('"').to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ga4gh_types::{AdsResourceType, DatasetVisibility};

    fn sample_dataset() -> Dataset {
        Dataset {
            id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            name: "Heidelberg Cancer Cohort".into(),
            description: None,
            external_id: Some("HGNC-001".into()),
            dac_group: Some("dac-onco".into()),
            duo_codes: vec![],
            auto_approve_enabled: false,
            auto_approve_threshold: 80,
            visibility: DatasetVisibility::Institute,
            resource_type: AdsResourceType::Dataset,
            remote_drs_base_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn event(event_type: AdsEventType, payload: BTreeMap<String, serde_json::Value>) -> AdsEvent {
        AdsEvent {
            id: Uuid::new_v4(),
            event_type,
            occurred_at: Utc::now(),
            payload,
        }
    }

    #[test]
    fn grant_created_label_uses_dataset_name() {
        let ctx = EventLabelContext::from_catalog(&[sample_dataset()], &[], &[]);
        let mut payload = BTreeMap::new();
        payload.insert("researcher_id".into(), serde_json::json!("alice@uni.de"));
        payload.insert(
            "dataset_id".into(),
            serde_json::json!("11111111-1111-1111-1111-111111111111"),
        );
        let label = format_event_label(&event(AdsEventType::GrantCreated, payload), &ctx);
        assert!(label.contains("alice@uni.de"));
        assert!(label.contains("Heidelberg Cancer Cohort"));
        assert!(label.contains("HGNC-001"));
        assert!(!label.contains("11111111"));
    }

    #[test]
    fn request_approved_label_uses_dataset_and_actor() {
        let ctx = EventLabelContext::from_catalog(&[sample_dataset()], &[], &[]);
        let mut payload = BTreeMap::new();
        payload.insert(
            "dataset_id".into(),
            serde_json::json!("11111111-1111-1111-1111-111111111111"),
        );
        payload.insert("researcher_id".into(), serde_json::json!("bob@uni.de"));
        payload.insert("actor".into(), serde_json::json!("dac-member-1"));
        let label = format_event_label(&event(AdsEventType::RequestApproved, payload), &ctx);
        assert!(label.contains("Heidelberg Cancer Cohort"));
        assert!(label.contains("bob@uni.de"));
        assert!(label.contains("dac-member-1"));
    }

    #[test]
    fn grant_revoked_resolves_via_grant_catalog() {
        let dataset_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let grant_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
        let grant = Grant {
            id: grant_id,
            researcher_id: "alice@uni.de".into(),
            dataset_id,
            source: ga4gh_types::GrantSource::DacApproval,
            duo_codes: vec![],
            request_id: None,
            resource_scope: None,
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
        };
        let ctx = EventLabelContext::from_catalog(&[sample_dataset()], &[], &[grant]);
        let mut payload = BTreeMap::new();
        payload.insert("grant_id".into(), serde_json::json!(grant_id));
        let label = format_event_label(&event(AdsEventType::GrantRevoked, payload), &ctx);
        assert!(label.contains("Heidelberg Cancer Cohort"));
        assert!(label.contains("alice@uni.de"));
    }
}
