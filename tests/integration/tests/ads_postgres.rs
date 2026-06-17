//! ADS institutional mapping and researcher sync against ephemeral PostgreSQL.

use std::collections::BTreeMap;

use access_decision_service::config::{DatabaseConfig, DatabaseDriver};
use access_decision_service::permissions::sync_researcher;
use access_decision_service::store::AdsStore;
use ga4gh_types::{
    CreateDatasetRequest, CreatePermissionMappingRequest, CreatePermissionSourceRequest, DuoCode,
    ResearcherSyncRequest,
};
use serde_json::json;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires Docker (testcontainers)"]
async fn postgres_sync_applies_institutional_mapping() {
    let container = Postgres::default().start().await.expect("start postgres");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let database = DatabaseConfig {
        driver: DatabaseDriver::Postgres,
        url: Some(url.clone()),
        url_env: "UNUSED".to_string(),
        auto_migrate: true,
    };

    let store = AdsStore::connect(&database, &url, vec![])
        .await
        .expect("connect postgres");

    let dataset = store
        .create_dataset(&CreateDatasetRequest {
            name: "Integration cohort".to_string(),
            description: None,
            duo_codes: vec![DuoCode::Gru],
            external_id: Some("dataset-integration".to_string()),
            auto_approve_enabled: false,
            auto_approve_threshold: 100,
            dac_group: Some("ega-dac".to_string()),
            visibility: ga4gh_types::DatasetVisibility::Institute,
            resource_type: ga4gh_types::AdsResourceType::Dataset,
            remote_drs_base_url: None,
        })
        .await
        .expect("create dataset");

    let source = store
        .create_permission_source(&CreatePermissionSourceRequest {
            name: "idp-groups".to_string(),
            oidc_issuer: "https://idp.example.org".to_string(),
            claim_path: "groups".to_string(),
        })
        .await
        .expect("create source");

    store
        .create_permission_mapping(&CreatePermissionMappingRequest {
            source_id: source.id,
            claim_value: "ega-approved".to_string(),
            dataset_id: dataset.id,
            grant_lifetime_seconds: Some(3600),
        })
        .await
        .expect("create mapping");

    let sub = format!("researcher-{}", Uuid::new_v4());
    let mut claims = BTreeMap::new();
    claims.insert("groups".to_string(), json!(["ega-approved"]));

    let grants = sync_researcher(
        &store,
        &ResearcherSyncRequest {
            sub: sub.clone(),
            display_name: Some("Integration Researcher".to_string()),
            email: Some("researcher@example.org".to_string()),
            claims,
            affiliations: vec![],
        },
    )
    .await
    .expect("sync researcher");

    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].researcher_id, sub);
    assert_eq!(grants[0].dataset_id, dataset.id);

    let researcher = store.get_researcher(&sub).await.expect("get researcher");
    assert_eq!(researcher.email.as_deref(), Some("researcher@example.org"));
}
