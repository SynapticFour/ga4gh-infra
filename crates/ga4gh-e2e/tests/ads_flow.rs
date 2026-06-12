// SPDX-License-Identifier: Apache-2.0

//! ADS grant and introspection end-to-end test against the docker stack.

mod support;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde_json::json;
use support::{
    ads_api_key, ads_url, broker_login, sample_resource_url, wait_for_service,
};

#[tokio::test]
#[ignore = "requires docker compose stack with ADS and sample-resource ads config"]
async fn stack_ads_grant_authorizes_via_introspect() {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("client");

    wait_for_service(&client, &ads_url()).await;
    wait_for_service(&client, &sample_resource_url()).await;

    let dataset_id = "dataset-registered-access-demo";
    let (subject, passport_jwt) = broker_login(&client).await;

    let dataset_response = client
        .post(format!("{}/ads/v1/datasets", ads_url()))
        .header("X-API-Key", ads_api_key())
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "name": "Registered Access Demo Cohort",
            "duo_codes": ["GRU", "NPU"],
            "external_id": dataset_id,
            "auto_approve_enabled": false,
            "auto_approve_threshold": 100
        }))
        .send()
        .await
        .expect("create ads dataset");
    assert!(
        dataset_response.status().is_success(),
        "create dataset failed: {}",
        dataset_response.status()
    );
    let ads_dataset = dataset_response
        .json::<serde_json::Value>()
        .await
        .expect("dataset json");
    let ads_dataset_id = ads_dataset["id"]
        .as_str()
        .expect("dataset id")
        .to_string();

    let project_response = client
        .post(format!("{}/ads/v1/projects", ads_url()))
        .header(AUTHORIZATION, format!("Bearer {passport_jwt}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "researcher_id": subject,
            "name": "E2E registered access project",
            "duo_codes": ["HMB", "NPU"]
        }))
        .send()
        .await
        .expect("create project");
    assert!(
        project_response.status().is_success(),
        "create project failed: {}",
        project_response.status()
    );
    let project_id = project_response
        .json::<serde_json::Value>()
        .await
        .expect("project json")["id"]
        .as_str()
        .expect("project id")
        .to_string();

    let request_response = client
        .post(format!("{}/ads/v1/access-requests", ads_url()))
        .header(AUTHORIZATION, format!("Bearer {passport_jwt}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "researcher_id": subject,
            "dataset_id": ads_dataset_id,
            "project_id": project_id,
            "justification": "E2E ADS grant flow"
        }))
        .send()
        .await
        .expect("create access request");
    assert!(
        request_response.status().is_success(),
        "create access request failed: {}",
        request_response.status()
    );
    let request_id = request_response
        .json::<serde_json::Value>()
        .await
        .expect("request json")["id"]
        .as_str()
        .expect("request id")
        .to_string();

    let approve_response = client
        .post(format!(
            "{}/ads/v1/dac/requests/{request_id}/approve",
            ads_url()
        ))
        .header("X-API-Key", ads_api_key())
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({ "reason": "approved for e2e" }))
        .send()
        .await
        .expect("approve request");
    assert_eq!(approve_response.status(), StatusCode::OK);

    let introspect_response = client
        .post(format!("{}/ads/v1/introspect", ads_url()))
        .header("X-API-Key", ads_api_key())
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "token": passport_jwt,
            "resource": dataset_id,
            "action": "read"
        }))
        .send()
        .await
        .expect("introspect");
    assert!(introspect_response.status().is_success());
    let introspect = introspect_response
        .json::<serde_json::Value>()
        .await
        .expect("introspect json");
    assert_eq!(introspect["active"], true);
    assert_eq!(introspect["sub"], subject);

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
}
