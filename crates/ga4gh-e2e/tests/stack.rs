// SPDX-License-Identifier: Apache-2.0

//! End-to-end stack test against a running docker-compose deployment.
//!
//! Run with:
//! ```text
//! docker compose -f docker/docker-compose.yml up --build --wait
//! cargo test -p ga4gh-e2e -- --ignored --test-threads=1
//! ```

mod support;

use std::time::Duration;

use ga4gh_clearinghouse::{Clearinghouse, ClearinghouseConfig, PolicyCheck, TrustedBroker};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde_json::json;
use support::{
    broker_login, broker_url, duo_service_url, sample_resource_url, visa_api_key,
    visa_registry_url, wait_for_service,
};

#[tokio::test]
#[ignore = "requires docker compose stack (see docs/architecture.md)"]
async fn stack_authenticates_passport_and_evaluates_policies() {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("client");

    wait_for_service(&client, &broker_url()).await;
    wait_for_service(&client, &visa_registry_url()).await;
    wait_for_service(&client, &duo_service_url()).await;
    wait_for_service(&client, &sample_resource_url()).await;

    let subject = "researcher@uni-heidelberg.de";
    let dataset_id = "dataset-registered-access-demo";

    let visa_response = client
        .post(format!("{}/visas", visa_registry_url()))
        .header("X-API-Key", visa_api_key())
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "sub": subject,
            "type": "ControlledAccessGrants",
            "value": dataset_id,
            "source": "https://dac.example.org",
            "by": "dac"
        }))
        .send()
        .await
        .expect("create visa");
    assert_eq!(visa_response.status(), StatusCode::CREATED);

    let (subject, passport_jwt) = broker_login(&client).await;

    let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
        vec![
            TrustedBroker::new(broker_url(), format!("{}/jwks.json", broker_url())),
            TrustedBroker::new(
                visa_registry_url(),
                format!("{}/jwks.json", visa_registry_url()),
            ),
        ],
        Duration::from_secs(300),
    ))
    .await
    .expect("clearinghouse");

    let passport = clearinghouse
        .validate_passport(&passport_jwt)
        .await
        .expect("validate passport");
    assert_eq!(passport.sub, subject);

    let visas = clearinghouse
        .extract_visas(&passport)
        .await
        .expect("extract visas");
    assert!(!visas.is_empty());

    let controlled = clearinghouse.check_policy(
        &visas,
        &PolicyCheck::HasControlledAccess {
            dataset_id: dataset_id.to_string(),
        },
    );
    assert!(controlled.permitted);

    let duo_match = client
        .post(format!("{}/match", duo_service_url()))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "dataset_duo": ["GRU", "NPU"],
            "intended_use": ["HMB", "NPU"]
        }))
        .send()
        .await
        .expect("duo match");
    assert!(duo_match.status().is_success());
    let duo_body = duo_match
        .json::<serde_json::Value>()
        .await
        .expect("duo json");
    assert_eq!(duo_body["permitted"], true);

    let dataset = client
        .get(format!("{}/datasets/{dataset_id}", sample_resource_url()))
        .header(AUTHORIZATION, format!("Bearer {passport_jwt}"))
        .send()
        .await
        .expect("sample resource dataset");
    let dataset_status = dataset.status();
    let dataset_body = dataset.text().await.expect("dataset body");
    assert!(
        dataset_status.is_success(),
        "sample resource dataset failed: HTTP {dataset_status} body={dataset_body}"
    );
    let dataset_json: serde_json::Value =
        serde_json::from_str(&dataset_body).expect("dataset json");
    assert_eq!(dataset_json["subject"], subject);
    assert_eq!(dataset_json["id"], dataset_id);

    let summary = client
        .get(format!(
            "{}/datasets/{dataset_id}/summary",
            sample_resource_url()
        ))
        .header(AUTHORIZATION, format!("Bearer {passport_jwt}"))
        .header("X-GA4GH-Intended-Use", "HMB,NPU")
        .send()
        .await
        .expect("sample resource summary");
    let summary_status = summary.status();
    let summary_body = summary.text().await.expect("summary body");
    assert!(
        summary_status.is_success(),
        "sample resource summary failed: HTTP {summary_status} body={summary_body}"
    );
    let summary_json: serde_json::Value =
        serde_json::from_str(&summary_body).expect("summary json");
    assert_eq!(summary_json["duo_permitted"], true);

    let userinfo = client
        .get(format!("{}/userinfo", broker_url()))
        .header(AUTHORIZATION, format!("Bearer {passport_jwt}"))
        .send()
        .await
        .expect("userinfo");
    assert!(userinfo.status().is_success());
}
