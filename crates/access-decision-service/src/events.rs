// SPDX-License-Identifier: Apache-2.0

//! Audit event emission and webhook delivery for ADS.

use std::collections::BTreeMap;

use chrono::Utc;
use ga4gh_types::{AccessRequest, AdsEvent, AdsEventType, Grant};
use reqwest::Client;
use serde_json::json;
use tracing::warn;
use uuid::Uuid;

use crate::error::AdsError;
use crate::store::AdsStore;

/// Persist an audit event, notify webhooks, and return the record.
pub async fn emit_event(
    store: &AdsStore,
    event_type: AdsEventType,
    payload: BTreeMap<String, serde_json::Value>,
) -> Result<AdsEvent, AdsError> {
    let event = AdsEvent {
        id: Uuid::new_v4(),
        event_type,
        occurred_at: Utc::now(),
        payload,
    };
    store.insert_event(&event).await?;
    notify_webhooks(store.webhook_urls(), &event).await;
    tracing::info!(event_type = ?event.event_type, event_id = %event.id, "ads audit event");
    Ok(event)
}

fn insert_dac_group(payload: &mut BTreeMap<String, serde_json::Value>, dac_group: Option<&str>) {
    if let Some(group) = dac_group {
        payload.insert("dac_group".to_string(), json!(group));
    }
}

async fn notify_webhooks(urls: &[String], event: &AdsEvent) {
    if urls.is_empty() {
        return;
    }
    let client = match Client::builder().use_rustls_tls().build() {
        Ok(client) => client,
        Err(err) => {
            warn!(error = %err, "webhook client build failed");
            return;
        }
    };
    for url in urls {
        if let Err(err) = client.post(url).json(event).send().await {
            warn!(%url, error = %err, "webhook delivery failed");
        }
    }
}

pub async fn grant_created(
    store: &AdsStore,
    grant_id: Uuid,
    researcher_id: &str,
    dataset_id: Uuid,
    dac_group: Option<&str>,
) -> Result<AdsEvent, AdsError> {
    let mut payload = BTreeMap::new();
    payload.insert("grant_id".to_string(), json!(grant_id));
    payload.insert("researcher_id".to_string(), json!(researcher_id));
    payload.insert("dataset_id".to_string(), json!(dataset_id));
    insert_dac_group(&mut payload, dac_group);
    emit_event(store, AdsEventType::GrantCreated, payload).await
}

pub async fn grant_revoked(store: &AdsStore, grant: &Grant) -> Result<AdsEvent, AdsError> {
    let mut payload = BTreeMap::new();
    payload.insert("grant_id".to_string(), json!(grant.id));
    payload.insert("researcher_id".to_string(), json!(grant.researcher_id));
    payload.insert("dataset_id".to_string(), json!(grant.dataset_id));
    emit_event(store, AdsEventType::GrantRevoked, payload).await
}

pub async fn request_created(
    store: &AdsStore,
    request_id: Uuid,
    researcher_id: &str,
    dataset_id: Uuid,
    dac_group: Option<&str>,
) -> Result<AdsEvent, AdsError> {
    let mut payload = BTreeMap::new();
    payload.insert("request_id".to_string(), json!(request_id));
    payload.insert("researcher_id".to_string(), json!(researcher_id));
    payload.insert("dataset_id".to_string(), json!(dataset_id));
    insert_dac_group(&mut payload, dac_group);
    emit_event(store, AdsEventType::RequestCreated, payload).await
}

pub async fn request_approved(
    store: &AdsStore,
    request: &AccessRequest,
    actor: &str,
) -> Result<AdsEvent, AdsError> {
    let mut payload = BTreeMap::new();
    payload.insert("request_id".to_string(), json!(request.id));
    payload.insert("researcher_id".to_string(), json!(request.researcher_id));
    payload.insert("dataset_id".to_string(), json!(request.dataset_id));
    payload.insert("project_id".to_string(), json!(request.project_id));
    payload.insert("actor".to_string(), json!(actor));
    insert_dac_group(&mut payload, request.dac_group.as_deref());
    emit_event(store, AdsEventType::RequestApproved, payload).await
}

pub async fn request_rejected(
    store: &AdsStore,
    request: &AccessRequest,
    actor: &str,
) -> Result<AdsEvent, AdsError> {
    let mut payload = BTreeMap::new();
    payload.insert("request_id".to_string(), json!(request.id));
    payload.insert("researcher_id".to_string(), json!(request.researcher_id));
    payload.insert("dataset_id".to_string(), json!(request.dataset_id));
    payload.insert("project_id".to_string(), json!(request.project_id));
    payload.insert("actor".to_string(), json!(actor));
    insert_dac_group(&mut payload, request.dac_group.as_deref());
    emit_event(store, AdsEventType::RequestRejected, payload).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, DatabaseDriver};
    use crate::store::AdsStore;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn delivers_event_to_configured_webhook() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let store = AdsStore::connect(
            &DatabaseConfig {
                driver: DatabaseDriver::Sqlite,
                url: Some("sqlite::memory:".to_string()),
                url_env: "ADS_DATABASE_URL".to_string(),
                auto_migrate: true,
            },
            "sqlite::memory:",
            vec![format!("{}/hook", server.uri())],
        )
        .await
        .expect("store");

        emit_event(
            &store,
            AdsEventType::GrantCreated,
            BTreeMap::from([("grant_id".to_string(), json!("abc"))]),
        )
        .await
        .expect("emit");

        server.verify().await;
    }
}
