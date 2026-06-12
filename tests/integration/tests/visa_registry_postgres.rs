//! Visa registry CRUD against ephemeral PostgreSQL (testcontainers).

use ga4gh_types::{VisaAuthority, VisaType};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;
use visa_registry::config::{DatabaseConfig, DatabaseDriver};
use visa_registry::store::{NewVisaAssertion, VisaStore};

fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers)"]
async fn postgres_crud_lifecycle_via_testcontainers() {
    let _ = fixtures_dir(); // ensures fixtures tree exists in CI checkout
    let container = Postgres::default().start().await.expect("start postgres");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let database = DatabaseConfig {
        driver: DatabaseDriver::Postgres,
        url: Some(url.clone()),
        url_env: "UNUSED".to_string(),
        auto_migrate: true,
    };

    let store = VisaStore::connect(&database, &url)
        .await
        .expect("connect postgres");

    let sub = format!("researcher-{}", Uuid::new_v4());
    let created = store
        .create_assertion(NewVisaAssertion {
            sub: sub.clone(),
            visa_type: VisaType::ControlledAccessGrants,
            value: "dataset-integration".to_string(),
            source: "https://dac.example.org".to_string(),
            by: Some(VisaAuthority::Dac),
            conditions: None,
            asserted: 1_700_000_000,
            expires_at: None,
        })
        .await
        .expect("create");

    let active = store.list_active_for_sub(&sub).await.expect("list");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, created.id);

    store.revoke_assertion(created.id).await.expect("revoke");
    assert!(store
        .list_active_for_sub(&sub)
        .await
        .expect("list")
        .is_empty());
}
