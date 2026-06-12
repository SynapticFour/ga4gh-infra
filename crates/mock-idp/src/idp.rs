// SPDX-License-Identifier: Apache-2.0

//! Mock OIDC endpoints with PKCE-aware token exchange.

use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Redirect;
use axum::Json;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::pkcs8::DecodePrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use url::Url;

/// Shared mock IdP state.
pub struct MockIdpState {
    issuer: String,
    subject: String,
    client_id: String,
    client_secret: String,
    encoding_key: EncodingKey,
    kid: String,
    jwks: Value,
    last_nonce: RwLock<Option<String>>,
    last_code_challenge: RwLock<Option<String>>,
}

impl MockIdpState {
    /// Load signing material and issuer configuration.
    pub fn new(issuer: &str, key_path: &str, subject: String) -> anyhow::Result<Self> {
        let pem = fs::read_to_string(key_path)
            .map_err(|err| anyhow::anyhow!("reading mock IdP key `{key_path}`: {err}"))?;
        let private_key = RsaPrivateKey::from_pkcs8_pem(&pem)?;
        let public_key = private_key.to_public_key();
        let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public_key.n().to_bytes_be()));
        let jwks = json!({
            "keys": [{
                "kty": "RSA",
                "kid": kid,
                "use": "sig",
                "alg": "RS256",
                "n": URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be()),
                "e": URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be()),
            }]
        });
        let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes())?;

        Ok(Self {
            issuer: issuer.to_string(),
            subject,
            client_id: std::env::var("MOCK_IDP_CLIENT_ID")
                .unwrap_or_else(|_| "ga4gh-broker".to_string()),
            client_secret: std::env::var("MOCK_IDP_CLIENT_SECRET")
                .unwrap_or_else(|_| "mock-client-secret".to_string()),
            encoding_key,
            kid,
            jwks,
            last_nonce: RwLock::new(None),
            last_code_challenge: RwLock::new(None),
        })
    }

    fn signing_header(&self) -> Header {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        header
    }
}

/// OIDC discovery document handler.
pub async fn openid_configuration(State(state): State<Arc<MockIdpState>>) -> Json<Value> {
    Json(json!({
        "issuer": state.issuer,
        "authorization_endpoint": format!("{}/oauth/authorize", state.issuer),
        "token_endpoint": format!("{}/oauth/token", state.issuer),
        "userinfo_endpoint": format!("{}/oauth/userinfo", state.issuer),
        "jwks_uri": format!("{}/jwks.json", state.issuer),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
        "scopes_supported": ["openid", "profile", "email"],
        "claims_supported": ["sub", "email", "name"],
    }))
}

/// JWKS document handler.
pub async fn jwks(State(state): State<Arc<MockIdpState>>) -> Json<Value> {
    Json(state.jwks.clone())
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    redirect_uri: String,
    state: String,
    nonce: Option<String>,
    code_challenge: Option<String>,
}

/// Authorization endpoint that immediately redirects back with an auth code.
pub async fn authorize(
    State(state): State<Arc<MockIdpState>>,
    Query(query): Query<AuthorizeQuery>,
) -> Result<Redirect, StatusCode> {
    let mut redirect = Url::parse(&query.redirect_uri).map_err(|_| StatusCode::BAD_REQUEST)?;
    {
        let mut pairs = redirect.query_pairs_mut();
        pairs.append_pair("code", "mock-auth-code");
        pairs.append_pair("state", &query.state);
    }

    if let Some(nonce) = query.nonce {
        *state.last_nonce.write().await = Some(nonce);
    }
    if let Some(challenge) = query.code_challenge {
        *state.last_code_challenge.write().await = Some(challenge);
    }

    Ok(Redirect::temporary(redirect.as_str()))
}

#[derive(Debug, Deserialize)]
pub struct TokenForm {
    grant_type: String,
    code: String,
    client_id: Option<String>,
    client_secret: Option<String>,
    code_verifier: Option<String>,
}

/// Token endpoint returning a signed ID token and access token.
pub async fn token(
    State(state): State<Arc<MockIdpState>>,
    headers: HeaderMap,
    form: axum::Form<TokenForm>,
) -> Result<Json<Value>, StatusCode> {
    if form.grant_type != "authorization_code" || form.code != "mock-auth-code" {
        return Err(StatusCode::BAD_REQUEST);
    }

    if !client_authenticated(&state, &headers, &form) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if let Some(challenge) = state.last_code_challenge.read().await.clone() {
        let verifier = form
            .code_verifier
            .as_deref()
            .ok_or(StatusCode::BAD_REQUEST)?;
        if !pkce_matches(verifier, &challenge) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let nonce = state
        .last_nonce
        .read()
        .await
        .clone()
        .unwrap_or_else(|| "mock-nonce".to_string());

    let now = unix_now();
    let id_token = encode(
        &state.signing_header(),
        &IdTokenClaims {
            iss: state.issuer.clone(),
            sub: state.subject.clone(),
            aud: state.client_id.clone(),
            exp: now + 3600,
            iat: now,
            nonce,
            email: Some(state.subject.clone()),
            name: Some("Test Researcher".to_string()),
        },
        &state.encoding_key,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "access_token": "mock-access-token",
        "token_type": "Bearer",
        "expires_in": 3600,
        "id_token": id_token,
        "scope": "openid profile email",
    })))
}

/// Userinfo endpoint for optional broker enrichment.
pub async fn userinfo(State(state): State<Arc<MockIdpState>>) -> Json<Value> {
    Json(json!({
        "sub": state.subject,
        "email": state.subject,
        "name": "Test Researcher",
        "preferred_username": state.subject,
    }))
}

#[derive(Debug, Serialize)]
struct IdTokenClaims {
    iss: String,
    sub: String,
    aud: String,
    exp: i64,
    iat: i64,
    nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

fn client_authenticated(state: &MockIdpState, headers: &HeaderMap, form: &TokenForm) -> bool {
    if let Some(header) = headers.get("authorization") {
        if let Ok(value) = header.to_str() {
            if let Some((client_id, client_secret)) = decode_basic_auth(value) {
                return client_id == state.client_id && client_secret == state.client_secret;
            }
        }
    }

    form.client_id.as_deref() == Some(state.client_id.as_str())
        && form.client_secret.as_deref() == Some(state.client_secret.as_str())
}

fn pkce_matches(verifier: &str, challenge: &str) -> bool {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest) == challenge
}

fn decode_basic_auth(header: &str) -> Option<(String, String)> {
    let encoded = header.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded).ok()?;
    let pair = String::from_utf8(decoded).ok()?;
    let (client_id, client_secret) = pair.split_once(':')?;
    Some((client_id.to_string(), client_secret.to_string()))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
