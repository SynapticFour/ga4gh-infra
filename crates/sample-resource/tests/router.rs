// SPDX-License-Identifier: Apache-2.0

//! Router integration tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use sample_resource::{build_router, AppState, SampleResourceConfig};
use tower::ServiceExt;

fn test_config() -> SampleResourceConfig {
    config::Config::builder()
        .add_source(config::File::from_str(
            r#"
            [server]
            host = "127.0.0.1"
            port = 8084
            external_url = "http://localhost:8084"
            environment = "development"

            [clearinghouse]
            jwks_cache_ttl_seconds = 300

            [[clearinghouse.trusted_issuers]]
            issuer = "http://localhost:8080"
            jwks_uri = "http://127.0.0.1:9/jwks.json"

            [duo_service]
            url = "http://127.0.0.1:9"

            [[datasets]]
            id = "dataset-registered-access-demo"
            name = "Demo"
            duo = ["GRU"]
            default_intended_use = ["HMB"]
            "#,
            config::FileFormat::Toml,
        ))
        .build()
        .expect("build config")
        .try_deserialize()
        .expect("parse config")
}

#[tokio::test]
async fn bare_path_param_router_works() {
    use axum::extract::Path;
    use axum::routing::get;
    use axum::Router;

    let app = Router::new().route("/hello/:name", get(async |Path(name): Path<String>| name));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hello/world")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn registers_dataset_routes() {
    let state = AppState::initialize(test_config())
        .await
        .expect("initialize");
    let app = build_router(state);

    let service_info = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/service-info")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("service-info response");
    assert_eq!(service_info.status(), StatusCode::OK);

    let health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("health response");
    assert_eq!(health.status(), StatusCode::OK);

    let probe = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/datasets/dataset-registered-access-demo")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("probe response");
    assert_ne!(
        probe.status(),
        StatusCode::NOT_FOUND,
        "resource route should be registered"
    );
}
