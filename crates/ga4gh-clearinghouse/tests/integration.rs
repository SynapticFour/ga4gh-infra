mod support;

use std::time::Duration;

use ga4gh_clearinghouse::{
    Clearinghouse, ClearinghouseConfig, ClearinghouseError, PolicyCheck, TrustedBroker,
};
use ga4gh_types::VisaType;
use serde_json::json;
use support::{TestIssuer, BROKER_ISSUER, VISA_ISSUER};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn validates_elixir_shaped_passport_and_visas() {
    let issuer = TestIssuer::new();
    let broker_mock = MockServer::start().await;
    let visa_mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/broker/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issuer.broker_jwks_json()))
        .mount(&broker_mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/visa/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issuer.visa_jwks_json()))
        .mount(&visa_mock)
        .await;

    let subject = "researcher@uni-heidelberg.de";
    let visa_jwts = issuer.elixir_shaped_visas(subject);
    let passport_jwt = issuer.mint_passport_jwt(subject, visa_jwts);

    let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
        vec![
            TrustedBroker::new(
                BROKER_ISSUER,
                format!("{}/broker/jwks.json", broker_mock.uri()),
            ),
            TrustedBroker::new(VISA_ISSUER, format!("{}/visa/jwks.json", visa_mock.uri())),
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
    assert_eq!(passport.visa_jwts.len(), 3);

    let visas = clearinghouse
        .extract_visas(&passport)
        .await
        .expect("extract visas");
    assert_eq!(visas.len(), 3);

    let controlled = clearinghouse.check_policy(
        &visas,
        &PolicyCheck::HasControlledAccess {
            dataset_id: "dataset-registered-access-demo".to_string(),
        },
    );
    assert!(controlled.permitted);

    let affiliation = clearinghouse.check_policy(
        &visas,
        &PolicyCheck::HasAffiliation {
            domain: "uni-heidelberg.de".to_string(),
        },
    );
    assert!(affiliation.permitted);
}

#[tokio::test]
async fn refreshes_jwks_when_kid_is_initially_unknown() {
    let issuer = TestIssuer::new();
    let mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": [] })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issuer.broker_jwks_json()))
        .expect(1)
        .mount(&mock)
        .await;

    let passport_jwt = issuer.mint_passport_jwt("researcher@example.org", vec![]);
    let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
        vec![TrustedBroker::new(
            BROKER_ISSUER,
            format!("{}/jwks.json", mock.uri()),
        )],
        Duration::from_secs(0),
    ))
    .await
    .expect("clearinghouse");

    clearinghouse
        .validate_passport(&passport_jwt)
        .await
        .expect("validate after refresh");
}

#[tokio::test]
async fn rejects_expired_passport() {
    let issuer = TestIssuer::new();
    let broker_mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/broker/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issuer.broker_jwks_json()))
        .mount(&broker_mock)
        .await;

    let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
        vec![TrustedBroker::new(
            BROKER_ISSUER,
            format!("{}/broker/jwks.json", broker_mock.uri()),
        )],
        Duration::from_secs(300),
    ))
    .await
    .expect("clearinghouse");

    let expired_passport = issuer.mint_passport_jwt_with_expiry("researcher@example.org", vec![], 1);
    let err = clearinghouse
        .validate_passport(&expired_passport)
        .await
        .expect_err("expired passport");
    assert!(matches!(err, ClearinghouseError::ExpiredPassport));
}

#[tokio::test]
async fn rejects_expired_visa() {
    let issuer = TestIssuer::new();
    let broker_mock = MockServer::start().await;
    let visa_mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/broker/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issuer.broker_jwks_json()))
        .mount(&broker_mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/visa/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issuer.visa_jwks_json()))
        .mount(&visa_mock)
        .await;

    let expired_visa = issuer.mint_visa_jwt_with_expiry(
        ga4gh_types::VisaClaim {
            r#type: VisaType::ControlledAccessGrants,
            asserted: 1_500_000_000,
            value: "dataset-old".to_string(),
            source: "https://visas.example.org".to_string(),
            by: None,
            conditions: None,
        },
        "researcher@example.org",
        "expired-visa",
        1_000_000_000,
    );

    let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
        vec![
            TrustedBroker::new(
                BROKER_ISSUER,
                format!("{}/broker/jwks.json", broker_mock.uri()),
            ),
            TrustedBroker::new(VISA_ISSUER, format!("{}/visa/jwks.json", visa_mock.uri())),
        ],
        Duration::from_secs(300),
    ))
    .await
    .expect("clearinghouse");

    let passport = clearinghouse
        .validate_passport(&issuer.mint_passport_jwt("researcher@example.org", vec![expired_visa]))
        .await
        .expect("passport");

    let err = clearinghouse
        .extract_visas(&passport)
        .await
        .expect_err("expired visa");
    assert!(matches!(err, ClearinghouseError::ExpiredVisa));
}
