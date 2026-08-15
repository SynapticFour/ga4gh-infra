// SPDX-License-Identifier: Apache-2.0

//! Upstream OIDC callback handler and passport issuance.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use openidconnect::core::CoreTokenResponse;
use openidconnect::TokenResponse;
use serde::Deserialize;
use serde_json::Value;
use tracing::instrument;

use crate::app::AppState;
use crate::error::BrokerError;
use crate::identity::ResearcherIdentity;
use crate::passport::mint_passport_jwt;
use crate::visas::collect_visas;

/// Query parameters returned by the upstream IdP authorization callback.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    /// Authorization code issued by the upstream IdP.
    pub code: Option<String>,
    /// CSRF state echoed by the upstream IdP.
    pub state: Option<String>,
    /// Upstream error code when authentication fails.
    pub error: Option<String>,
}

/// OAuth callback that completes the upstream login and mints a GA4GH Passport JWT.
#[instrument(skip(state, headers))]
pub async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
    headers: HeaderMap,
) -> Result<Response, BrokerError> {
    if query.error.is_some() || query.code.is_none() || query.state.is_none() {
        return Err(BrokerError::AuthenticationFailed);
    }
    if !state
        .login_limiter
        .allow(&ga4gh_http::client_key(&headers))
        .await
    {
        return Err(BrokerError::TooManyRequests);
    }

    let session = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_session_cookie)
        .and_then(|raw| state.sessions.parse_cookie_value(raw).ok())
        .ok_or(BrokerError::AuthenticationFailed)?;

    validate_csrf_state(&session, query.state.as_deref().unwrap_or_default())?;

    let idp = state.upstream.get(&session.idp_name)?;
    let nonce = session.nonce.as_deref().unwrap_or_default();
    let token_response = idp
        .exchange_code(
            &state.http_client,
            query.code.as_deref().unwrap_or_default(),
            &session.pkce_verifier,
            nonce,
        )
        .await?;

    let (identity, claims) =
        identity_from_token_response(&idp, &state.http_client, &token_response, nonce).await?;

    if let Some(ads) = &state.ads {
        ads.sync_researcher(&identity, &claims).await?;
    }

    let mut visas = collect_visas(&state.visa_sources, &identity.sub).await?;
    if let Some(ads) = &state.ads {
        let mut signed = ads.fetch_signed_visas(&identity.sub).await?;
        visas.append(&mut signed);
    }
    let minted = mint_passport_jwt(
        &state.keys,
        state.config.issuer_url(),
        &identity,
        &visas,
        state.config.signing.passport_lifetime_seconds,
    )?;
    let visa_jtis: Vec<String> = visas
        .iter()
        .filter_map(|jwt| {
            jwt_payload_value(jwt)?
                .get("jti")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .collect();
    state
        .passport_ledger
        .record_issue(
            minted.jti.clone(),
            identity.sub.clone(),
            minted.exp,
            visa_jtis,
        )
        .await?;
    let passport_jwt = minted.jwt;
    let exp = minted.exp;
    state.profiles.insert(&identity, exp).await;

    let clear_cookie = state.sessions.clear_set_cookie();
    let return_url = session.return_url.clone();
    let expires_in = state.config.signing.passport_lifetime_seconds;
    tracing::info!(
        audit = true,
        event = "passport.issued",
        sub = %identity.sub,
        idp = %session.idp_name,
        visa_count = visas.len(),
        expires_in,
        return_url_set = session.return_url.is_some(),
        client_ip = %ga4gh_http::client_key(&headers),
        "passport issued"
    );
    let body = serde_json::json!({
        "access_token": passport_jwt,
        "token_type": "Bearer",
        "expires_in": expires_in,
        "scope": "openid ga4gh_passport_v1",
    });

    let mut response = if let Some(return_url) = return_url {
        let fragment = format!(
            "access_token={}&token_type=Bearer&expires_in={}",
            urlencoding::encode(&passport_jwt),
            expires_in
        );
        let location = format!("{return_url}#{fragment}");
        Redirect::temporary(&location).into_response()
    } else if headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("application/json"))
    {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (
            StatusCode::OK,
            "Authentication successful. Use the access token as a Bearer token with /userinfo.\n",
        )
            .into_response()
    };

    response.headers_mut().append(
        header::SET_COOKIE,
        clear_cookie
            .parse()
            .map_err(|err| BrokerError::Internal(format!("invalid Set-Cookie header: {err}")))?,
    );

    Ok(response)
}

fn validate_csrf_state(
    session: &crate::session::RpSession,
    returned_state: &str,
) -> Result<(), BrokerError> {
    if returned_state != session.csrf_state {
        return Err(BrokerError::AuthenticationFailed);
    }
    Ok(())
}

fn parse_session_cookie(raw: &str) -> Option<&str> {
    raw.split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("ga4gh_broker_rp_session="))
}

async fn identity_from_token_response(
    idp: &crate::upstream::UpstreamIdp,
    http_client: &reqwest::Client,
    token_response: &CoreTokenResponse,
    nonce: &str,
) -> Result<(ResearcherIdentity, BTreeMap<String, Value>), BrokerError> {
    let id_token = token_response
        .id_token()
        .ok_or(BrokerError::AuthenticationFailed)?;
    let claims = id_token
        .claims(
            &idp.client.id_token_verifier(),
            &openidconnect::Nonce::new(nonce.to_string()),
        )
        .map_err(|_| BrokerError::AuthenticationFailed)?;

    let mut claim_map = HashMap::new();
    claim_map.insert(
        "sub".to_string(),
        Value::String(claims.subject().as_str().to_string()),
    );
    if let Some(email) = claims.email() {
        claim_map.insert(
            "email".to_string(),
            Value::String(email.as_str().to_string()),
        );
    }
    if let Some(name) = claims.name().and_then(|value| value.get(None)) {
        claim_map.insert("name".to_string(), Value::String(name.to_string()));
    }
    if let Some(payload) = jwt_payload_value(&id_token.to_string()) {
        if let Some(groups) = payload.get("groups") {
            claim_map.insert("groups".to_string(), groups.clone());
        }
    }

    if let Some(userinfo) = idp.fetch_userinfo(http_client, token_response).await? {
        if let Some(email) = userinfo.email() {
            claim_map.insert(
                "email".to_string(),
                Value::String(email.as_str().to_string()),
            );
        }
        if let Some(name) = userinfo.name().and_then(|value| value.get(None)) {
            claim_map.insert("name".to_string(), Value::String(name.to_string()));
        }
        if let Some(preferred) = userinfo.preferred_username() {
            claim_map.insert(
                "preferred_username".to_string(),
                Value::String(preferred.as_str().to_string()),
            );
        }
    }

    ResearcherIdentity::from_upstream(&idp.config, &claim_map)
        .ok_or(BrokerError::AuthenticationFailed)
        .map(|identity| (identity, BTreeMap::from_iter(claim_map)))
}

fn jwt_payload_value(token: &str) -> Option<Value> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_session_cookie_value() {
        let raw = "foo=bar; ga4gh_broker_rp_session=signed-value; Path=/";
        assert_eq!(parse_session_cookie(raw), Some("signed-value"));
    }

    #[test]
    fn rejects_mismatched_csrf_state() {
        use crate::session::{unix_now, RpSession};

        let session = RpSession {
            idp_name: "mock-idp".to_string(),
            csrf_state: "expected-state".to_string(),
            pkce_verifier: "verifier".to_string(),
            nonce: None,
            created_at: unix_now(),
            return_url: None,
        };

        assert!(validate_csrf_state(&session, "expected-state").is_ok());
        assert!(validate_csrf_state(&session, "wrong-state").is_err());
    }
}
