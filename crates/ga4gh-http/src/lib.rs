// SPDX-License-Identifier: Apache-2.0

//! Shared HTTP hardening: security headers, rate limits, and API-key hashing.

mod api_key;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tokio::sync::Mutex;

pub use api_key::{constant_time_eq, hash_api_key, lookup_hashes, verify_api_key};

/// Liveness probe used by every HTTP service (`GET /health`).
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Attach baseline security headers to every response.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let cacheable = {
        let path = request.uri().path();
        path.ends_with("/jwks.json")
            || path.ends_with("/revoked-jtis")
            || path.ends_with("/revoked-passports")
    };
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );
    if cacheable {
        headers
            .entry(header::CACHE_CONTROL)
            .or_insert(HeaderValue::from_static("public, max-age=60"));
    } else {
        headers
            .entry(header::CACHE_CONTROL)
            .or_insert(HeaderValue::from_static("no-store"));
    }
    response
}

/// Client identity used as a rate-limit key (`X-Forwarded-For` / `X-Real-IP`).
pub fn client_key(headers: &HeaderMap) -> String {
    if let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        if let Some(first) = forwarded.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    if let Some(real_ip) = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
    {
        let trimmed = real_ip.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "unknown".to_string()
}

/// In-memory sliding window limiter (per-process; use the reverse proxy for multi-replica).
#[derive(Clone)]
pub struct SlidingWindowLimiter {
    max: u32,
    window: Duration,
    hits: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl SlidingWindowLimiter {
    /// Create a limiter. `max == 0` disables enforcement.
    pub fn new(max_per_window: u32, window: Duration) -> Self {
        Self {
            max: max_per_window,
            window,
            hits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Return whether `key` is currently allowed.
    pub async fn allow(&self, key: &str) -> bool {
        if self.max == 0 {
            return true;
        }
        let now = Instant::now();
        let mut hits = self.hits.lock().await;
        let queue = hits.entry(key.to_string()).or_default();
        while queue
            .front()
            .is_some_and(|at| now.duration_since(*at) > self.window)
        {
            queue.pop_front();
        }
        if queue.len() as u32 >= self.max {
            return false;
        }
        queue.push_back(now);
        true
    }
}

/// Standard 429 body for public login endpoints.
pub fn too_many_requests() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        axum::Json(serde_json::json!({ "message": "Too many requests" })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn limiter_blocks_after_max() {
        let limiter = SlidingWindowLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.allow("a").await);
        assert!(limiter.allow("a").await);
        assert!(!limiter.allow("a").await);
        assert!(limiter.allow("b").await);
    }

    #[tokio::test]
    async fn limiter_disabled_when_max_zero() {
        let limiter = SlidingWindowLimiter::new(0, Duration::from_secs(60));
        for _ in 0..20 {
            assert!(limiter.allow("a").await);
        }
    }
}
