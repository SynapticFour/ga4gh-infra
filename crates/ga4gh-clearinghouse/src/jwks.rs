// SPDX-License-Identifier: Apache-2.0

//! JWKS fetching and caching for trusted issuers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tracing::instrument;

use crate::config::TrustedBroker;
use crate::error::ClearinghouseError;
use crate::token::{decode_jwt_header, peek_meta};

/// Cached JWKS keys keyed by `kid`.
#[derive(Clone)]
struct CachedJwks {
    keys_by_kid: HashMap<String, Arc<DecodingKey>>,
    fetched_at: Instant,
}

/// Fetch and cache JWKS documents for trusted issuers.
pub struct JwksCache {
    http: Client,
    ttl: Duration,
    brokers_by_issuer: HashMap<String, TrustedBroker>,
    cache: RwLock<HashMap<String, CachedJwks>>,
    inflight: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl JwksCache {
    /// Create a JWKS cache for the given trusted brokers.
    pub fn new(
        trusted_brokers: Vec<TrustedBroker>,
        ttl: Duration,
    ) -> Result<Self, ClearinghouseError> {
        let http = Client::builder()
            .use_rustls_tls()
            .build()
            .map_err(|err| ClearinghouseError::Internal(err.to_string()))?;

        let brokers_by_issuer = trusted_brokers
            .into_iter()
            .map(|broker| (broker.issuer.clone(), broker))
            .collect();

        Ok(Self {
            http,
            ttl,
            brokers_by_issuer,
            cache: RwLock::new(HashMap::new()),
            inflight: Mutex::new(HashMap::new()),
        })
    }

    /// Return the configured trusted broker for an issuer, if any.
    pub fn trusted_broker(&self, issuer: &str) -> Option<&TrustedBroker> {
        self.brokers_by_issuer.get(issuer)
    }

    /// Verify a JWT and deserialize its claims.
    #[instrument(skip(self, token))]
    pub async fn verify_and_decode<T>(&self, token: &str) -> Result<T, ClearinghouseError>
    where
        T: serde::de::DeserializeOwned,
    {
        let meta = peek_meta(token)?;
        if is_expired(meta.exp) {
            return Err(ClearinghouseError::ExpiredPassport);
        }
        let broker = self
            .trusted_broker(&meta.iss)
            .ok_or(ClearinghouseError::UntrustedIssuer)?;

        let header = decode_jwt_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| ClearinghouseError::UnknownKeyId("missing kid".to_string()))?;

        let key = self.decoding_key_for(&broker.jwks_uri, &kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[meta.iss.as_str()]);
        validation.validate_aud = false;

        decode::<T>(token, key.as_ref(), &validation)
            .map(|data| data.claims)
            .map_err(map_decode_error)
    }

    async fn decoding_key_for(
        &self,
        jwks_uri: &str,
        kid: &str,
    ) -> Result<Arc<DecodingKey>, ClearinghouseError> {
        if let Some(key) = self.cached_key(jwks_uri, kid).await {
            return Ok(key);
        }

        let gate = {
            let mut inflight = self.inflight.lock().await;
            inflight
                .entry(jwks_uri.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _in_flight = gate.lock().await;

        if let Some(key) = self.cached_key(jwks_uri, kid).await {
            return Ok(key);
        }

        let mut cache = self.cache.write().await;
        if let Some(key) = cached_key_from(&cache, jwks_uri, kid, self.ttl) {
            return Ok(key);
        }

        self.refresh_locked(&mut cache, jwks_uri).await?;
        if let Some(key) = cached_key_from(&cache, jwks_uri, kid, self.ttl) {
            return Ok(key);
        }

        // Retry only when the issuer returned an empty JWKS (startup / rotation race).
        let empty = cache
            .get(jwks_uri)
            .is_none_or(|entry| entry.keys_by_kid.is_empty());
        if empty {
            self.refresh_locked(&mut cache, jwks_uri).await?;
        }
        cached_key_from(&cache, jwks_uri, kid, self.ttl)
            .ok_or_else(|| ClearinghouseError::UnknownKeyId(kid.to_string()))
    }

    async fn cached_key(&self, jwks_uri: &str, kid: &str) -> Option<Arc<DecodingKey>> {
        let cache = self.cache.read().await;
        cached_key_from(&cache, jwks_uri, kid, self.ttl)
    }

    #[instrument(skip(self, cache))]
    async fn refresh_locked(
        &self,
        cache: &mut tokio::sync::RwLockWriteGuard<'_, HashMap<String, CachedJwks>>,
        jwks_uri: &str,
    ) -> Result<(), ClearinghouseError> {
        let response = self
            .http
            .get(jwks_uri)
            .send()
            .await
            .map_err(|err| ClearinghouseError::JwksFetchFailed(err.to_string()))?;

        if !response.status().is_success() {
            return Err(ClearinghouseError::JwksFetchFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let document = response
            .json::<JwksDocument>()
            .await
            .map_err(|err| ClearinghouseError::JwksFetchFailed(err.to_string()))?;

        let mut keys_by_kid = HashMap::new();
        for key in document.keys {
            let Some(kid) = key.kid else {
                continue;
            };
            if key.kty != "RSA" {
                continue;
            }
            let Some(n) = key.n else {
                continue;
            };
            let Some(e) = key.e else {
                continue;
            };
            let decoding_key = Arc::new(decoding_key_from_components(&n, &e)?);
            keys_by_kid.insert(kid, decoding_key);
        }

        cache.insert(
            jwks_uri.to_string(),
            CachedJwks {
                keys_by_kid,
                fetched_at: Instant::now(),
            },
        );
        Ok(())
    }
}

fn cached_key_from(
    cache: &HashMap<String, CachedJwks>,
    jwks_uri: &str,
    kid: &str,
    ttl: Duration,
) -> Option<Arc<DecodingKey>> {
    let entry = cache.get(jwks_uri)?;
    if ttl > Duration::ZERO && entry.fetched_at.elapsed() > ttl {
        return None;
    }
    entry.keys_by_kid.get(kid).cloned()
}

fn is_expired(exp: i64) -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
        >= exp
}

fn decoding_key_from_components(n: &str, e: &str) -> Result<DecodingKey, ClearinghouseError> {
    DecodingKey::from_rsa_components(n, e)
        .map_err(|err| ClearinghouseError::JwksFetchFailed(err.to_string()))
}

fn map_decode_error(err: jsonwebtoken::errors::Error) -> ClearinghouseError {
    match err.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => ClearinghouseError::ExpiredPassport,
        jsonwebtoken::errors::ErrorKind::InvalidSignature => ClearinghouseError::InvalidSignature,
        jsonwebtoken::errors::ErrorKind::InvalidIssuer => ClearinghouseError::UntrustedIssuer,
        _ => ClearinghouseError::InvalidClaims(err.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<JwkEntry>,
}

#[derive(Debug, Deserialize)]
struct JwkEntry {
    kid: Option<String>,
    kty: String,
    n: Option<String>,
    e: Option<String>,
}

/// Shared handle to a JWKS cache.
pub type SharedJwksCache = Arc<JwksCache>;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::config::TrustedBroker;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_broker(jwks_uri: String) -> TrustedBroker {
        TrustedBroker {
            issuer: "https://broker.example.org".to_string(),
            jwks_uri,
        }
    }

    #[tokio::test]
    async fn caches_decoding_key_after_successful_fetch() {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        use rand::SeedableRng;
        use rsa::traits::PublicKeyParts;
        use rsa::RsaPrivateKey;

        let private =
            RsaPrivateKey::new(&mut rand_chacha::ChaCha8Rng::seed_from_u64(7), 2048).expect("rsa");
        let public = private.to_public_key();
        let n = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());

        let server = MockServer::start().await;
        let jwks = serde_json::json!({
            "keys": [{
                "kid": "cache-test-kid",
                "kty": "RSA",
                "n": n,
                "e": e
            }]
        });
        Mock::given(method("GET"))
            .and(path("/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(jwks))
            .expect(1)
            .mount(&server)
            .await;

        let jwks_uri = format!("{}/jwks.json", server.uri());
        let cache = JwksCache::new(
            vec![test_broker(jwks_uri.clone())],
            Duration::from_secs(300),
        )
        .expect("cache");

        cache
            .decoding_key_for(&jwks_uri, "cache-test-kid")
            .await
            .expect("first fetch");
        cache
            .decoding_key_for(&jwks_uri, "cache-test-kid")
            .await
            .expect("cache hit");
    }

    #[tokio::test]
    async fn jwks_fetch_failure_returns_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jwks.json"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let cache = JwksCache::new(
            vec![test_broker(format!("{}/jwks.json", server.uri()))],
            Duration::from_secs(300),
        )
        .expect("cache");

        let result = cache
            .decoding_key_for(&format!("{}/jwks.json", server.uri()), "missing")
            .await;
        assert!(matches!(
            result,
            Err(ClearinghouseError::JwksFetchFailed(_))
        ));
    }
}
