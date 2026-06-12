// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for docker-stack end-to-end tests.

#![allow(dead_code)]

use std::time::Duration;

use reqwest::header::{ACCEPT, COOKIE, SET_COOKIE};
use reqwest::Client;

pub fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn broker_url() -> String {
    env_or("GA4GH_BROKER_URL", "http://localhost:8080")
}

pub fn visa_registry_url() -> String {
    env_or("GA4GH_VISA_REGISTRY_URL", "http://localhost:8081")
}

pub fn ads_url() -> String {
    env_or("GA4GH_ADS_URL", "http://localhost:8090")
}

pub fn duo_service_url() -> String {
    env_or("GA4GH_DUO_SERVICE_URL", "http://localhost:8082")
}

pub fn sample_resource_url() -> String {
    env_or("GA4GH_SAMPLE_RESOURCE_URL", "http://localhost:8084")
}

pub fn visa_api_key() -> String {
    env_or("GA4GH_VISA_API_KEY", "dev-visa-api-key")
}

pub fn ads_api_key() -> String {
    env_or("GA4GH_ADS_API_KEY", "dev-ads-api-key")
}

pub async fn wait_for_service(client: &Client, url: &str) {
    for _ in 0..60 {
        if client
            .get(format!("{url}/service-info"))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    panic!("service at {url} did not become healthy in time");
}

/// Complete broker login via mock-idp and return `(subject, passport_jwt)`.
pub async fn broker_login(client: &Client) -> (String, String) {
    let login = client
        .get(format!("{}/login", broker_url()))
        .header(ACCEPT, "application/json")
        .send()
        .await
        .expect("login");
    assert!(login.status().is_success());
    let session_cookie = login
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("ga4gh_broker_rp_session="))
        .expect("session cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string();
    let auth_url = login.json::<serde_json::Value>().await.expect("login json")
        ["authorization_url"]
        .as_str()
        .expect("authorization_url")
        .replace("mock-idp:9000", "localhost:9000");

    let auth_redirect = client.get(auth_url).send().await.expect("authorize");
    assert!(
        auth_redirect.status().is_redirection(),
        "expected redirect, got {}",
        auth_redirect.status()
    );
    let callback_url = auth_redirect
        .headers()
        .get("location")
        .expect("callback location")
        .to_str()
        .expect("location utf8")
        .to_string();

    let callback = client
        .get(callback_url)
        .header(ACCEPT, "application/json")
        .header(COOKIE, session_cookie)
        .send()
        .await
        .expect("callback");
    assert!(callback.status().is_success());
    let callback_json = callback
        .json::<serde_json::Value>()
        .await
        .expect("callback json");
    let passport_jwt = callback_json["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    let subject = env_or("MOCK_IDP_SUBJECT", "researcher@uni-heidelberg.de");
    (subject, passport_jwt)
}
