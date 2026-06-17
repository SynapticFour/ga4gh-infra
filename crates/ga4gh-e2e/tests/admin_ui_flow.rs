// SPDX-License-Identifier: Apache-2.0

//! Admin-ui DAC queue end-to-end test against the docker stack.

mod support;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, COOKIE};
use reqwest::Client;
use serde_json::json;
use support::{
    admin_ui_session, admin_ui_url, ads_api_key, ads_url, broker_login, wait_for_service,
};

#[tokio::test]
#[ignore = "requires docker compose stack with admin-ui and ADS"]
async fn stack_admin_ui_approves_dac_request_via_htmx() {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("client");

    wait_for_service(&client, &ads_url()).await;
    wait_for_service(&client, &admin_ui_url()).await;

    let dataset_external_id = format!("admin-ui-e2e-{}", uuid::Uuid::new_v4());
    let (subject, passport_jwt) = broker_login(&client).await;
    let session_cookie = admin_ui_session(&client, &passport_jwt).await;

    let dashboard = client
        .get(format!("{}/", admin_ui_url()))
        .header(COOKIE, &session_cookie)
        .send()
        .await
        .expect("dashboard");
    assert!(dashboard.status().is_success());
    let dashboard_html = dashboard.text().await.expect("dashboard html");
    assert!(dashboard_html.contains("Dashboard"));

    let dataset_response = client
        .post(format!("{}/ads/v1/datasets", ads_url()))
        .header("X-API-Key", ads_api_key())
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "name": "Admin UI E2E Dataset",
            "duo_codes": ["GRU", "NPU"],
            "external_id": dataset_external_id,
            "auto_approve_enabled": false,
            "auto_approve_threshold": 100
        }))
        .send()
        .await
        .expect("create dataset");
    assert!(dataset_response.status().is_success());
    let ads_dataset_id = dataset_response
        .json::<serde_json::Value>()
        .await
        .expect("dataset json")["id"]
        .as_str()
        .expect("dataset id")
        .to_string();

    let project_response = client
        .post(format!("{}/ads/v1/projects", ads_url()))
        .header(AUTHORIZATION, format!("Bearer {passport_jwt}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "researcher_id": subject,
            "name": "Admin UI E2E project",
            "duo_codes": ["HMB", "NPU"]
        }))
        .send()
        .await
        .expect("create project");
    assert!(project_response.status().is_success());
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
            "justification": "Admin UI e2e DAC approval"
        }))
        .send()
        .await
        .expect("create access request");
    assert!(request_response.status().is_success());
    let request_id = request_response
        .json::<serde_json::Value>()
        .await
        .expect("request json")["id"]
        .as_str()
        .expect("request id")
        .to_string();

    let queue_partial = client
        .get(format!("{}/dac/queue", admin_ui_url()))
        .header(COOKIE, &session_cookie)
        .send()
        .await
        .expect("dac queue partial");
    assert!(queue_partial.status().is_success());
    let queue_html = queue_partial.text().await.expect("queue html");
    assert!(
        queue_html.contains(&request_id),
        "queue partial should list pending request {request_id}"
    );

    let approve_response = client
        .post(format!(
            "{}/dac/requests/{request_id}/approve",
            admin_ui_url()
        ))
        .header(COOKIE, &session_cookie)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("HX-Request", "true")
        .body("reason=Admin+UI+e2e+DAC+approval")
        .send()
        .await
        .expect("approve via admin-ui");
    assert!(
        approve_response.status().is_success(),
        "approve failed: {}",
        approve_response.status()
    );
    let approve_html = approve_response.text().await.expect("approve row html");
    assert!(
        approve_html.contains(&request_id),
        "htmx approve should return updated queue row for {request_id}"
    );
    assert!(
        approve_html.contains("Approved"),
        "htmx approve row should show Approved status"
    );

    let grants_response = client
        .get(format!("{}/ads/v1/grants", ads_url()))
        .header("X-API-Key", ads_api_key())
        .query(&[("researcher_id", &subject)])
        .send()
        .await
        .expect("list grants");
    assert!(grants_response.status().is_success());
    let grants = grants_response
        .json::<serde_json::Value>()
        .await
        .expect("grants json");
    let matched = grants["grants"]
        .as_array()
        .expect("grants array")
        .iter()
        .any(|grant| {
            grant["researcher_id"].as_str() == Some(subject.as_str())
                && grant["dataset_id"].as_str() == Some(ads_dataset_id.as_str())
        });
    assert!(matched, "expected grant for approved request");
}
