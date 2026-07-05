//! Fetching and caching Google's public signing keys (JWKS) for ID token
//! verification.
//!
//! Keys are published at
//! `https://www.googleapis.com/service_accounts/v1/metadata/jwk/securetoken@system.gserviceaccount.com`.

use crate::auth::error::TokenVerificationError;
use crate::core::http::parse_cache_control_max_age;
use crate::core::HttpClient;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const SECURETOKEN_JWKS_URL: &str =
    "https://www.googleapis.com/service_accounts/v1/metadata/jwk/securetoken@system.gserviceaccount.com";

/// The minimum amount of time a cached key set is trusted before a refresh
/// is attempted, even if the response did not specify a `Cache-Control` max-age.
const MIN_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

struct Cached {
    keys: HashMap<String, Jwk>,
    fetched_at: Instant,
    ttl: Duration,
}

/// Fetches and caches Google's public keys used to verify Firebase ID tokens.
///
/// Concurrent lookups that miss the cache at the same time (e.g. a burst of
/// `verify_id_token` calls right after Google rotates its signing keys) share
/// a single in-flight refresh rather than each issuing their own HTTP
/// request: an internal lock is held for the duration of the fetch, so
/// callers that arrive while a refresh is already underway simply wait for
/// it to finish and then re-check the now-populated cache.
pub struct JwksCache {
    url: String,
    http: HttpClient,
    cache: RwLock<Option<Cached>>,
    refresh_lock: Mutex<()>,
}

impl JwksCache {
    /// Creates a cache pointed at the standard securetoken JWKS endpoint.
    pub fn new(http: HttpClient) -> Self {
        Self {
            url: SECURETOKEN_JWKS_URL.to_string(),
            http,
            cache: RwLock::new(None),
            refresh_lock: Mutex::new(()),
        }
    }

    /// Returns the RSA public key components (`n`, `e`, base64url-encoded)
    /// for the given key id, fetching or refreshing the cache as needed.
    pub async fn public_key(&self, kid: &str) -> Result<(String, String), TokenVerificationError> {
        if let Some((n, e)) = self.cached_key(kid) {
            return Ok((n, e));
        }

        let _guard = self.refresh_lock.lock().await;

        // Another caller may have refreshed the cache while we were waiting
        // for the lock; re-check before issuing a redundant request.
        if let Some((n, e)) = self.cached_key(kid) {
            return Ok((n, e));
        }

        self.refresh().await?;

        self.cached_key(kid)
            .ok_or(TokenVerificationError::InvalidSignature)
    }

    fn cached_key(&self, kid: &str) -> Option<(String, String)> {
        let guard = self.cache.read().ok()?;
        let cached = guard.as_ref()?;
        if cached.fetched_at.elapsed() > cached.ttl {
            return None;
        }
        cached
            .keys
            .get(kid)
            .map(|jwk| (jwk.n.clone(), jwk.e.clone()))
    }

    async fn refresh(&self) -> Result<(), TokenVerificationError> {
        let response = self
            .http
            .inner()
            .get(&self.url)
            .send()
            .await
            .map_err(|e| TokenVerificationError::Jwks(e.to_string()))?;

        let ttl = response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_cache_control_max_age)
            .unwrap_or(MIN_CACHE_TTL);

        let jwk_set: JwkSet = response
            .json()
            .await
            .map_err(|e| TokenVerificationError::Jwks(e.to_string()))?;

        let keys = jwk_set
            .keys
            .into_iter()
            .map(|jwk| (jwk.kid.clone(), jwk))
            .collect();

        let mut guard = self
            .cache
            .write()
            .map_err(|_| TokenVerificationError::Jwks("jwks cache lock poisoned".to_string()))?;
        *guard = Some(Cached {
            keys,
            fetched_at: Instant::now(),
            ttl,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_jwk_set_body() -> serde_json::Value {
        serde_json::json!({
            "keys": [
                {
                    "kid": "key-1",
                    "kty": "RSA",
                    "alg": "RS256",
                    "use": "sig",
                    "n": "test-n-value",
                    "e": "AQAB",
                }
            ]
        })
    }

    async fn cache_pointed_at(server: &MockServer) -> JwksCache {
        let http = HttpClient::default();
        JwksCache {
            url: format!("{}/jwks", server.uri()),
            http,
            cache: RwLock::new(None),
            refresh_lock: Mutex::new(()),
        }
    }

    #[tokio::test]
    async fn fetches_and_returns_a_known_key() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_jwk_set_body()))
            .expect(1)
            .mount(&server)
            .await;

        let cache = cache_pointed_at(&server).await;
        let (n, e) = cache.public_key("key-1").await.unwrap();
        assert_eq!(n, "test-n-value");
        assert_eq!(e, "AQAB");
    }

    #[tokio::test]
    async fn unknown_kid_after_refresh_is_invalid_signature() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_jwk_set_body()))
            .mount(&server)
            .await;

        let cache = cache_pointed_at(&server).await;
        let err = cache.public_key("does-not-exist").await.unwrap_err();
        assert!(matches!(err, TokenVerificationError::InvalidSignature));
    }

    #[tokio::test]
    async fn a_second_lookup_uses_the_cache_and_does_not_refetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_jwk_set_body()))
            .expect(1)
            .mount(&server)
            .await;

        let cache = cache_pointed_at(&server).await;
        cache.public_key("key-1").await.unwrap();
        cache.public_key("key-1").await.unwrap();
    }

    #[tokio::test]
    async fn an_expired_cache_entry_triggers_a_refetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_jwk_set_body()))
            .expect(2) // one fetch to seed, one to observe the actual refetch
            .mount(&server)
            .await;

        let cache = cache_pointed_at(&server).await;

        // Seed the cache normally, then shrink its TTL to zero so the entry
        // reads as already past its TTL — proving `cached_key` treats a
        // stale entry as a miss and `public_key` refetches, not just that a
        // cold cache fetches once. Shrinking `ttl` (rather than backdating
        // `fetched_at` by subtracting from `Instant::now()`) avoids a
        // platform-dependent underflow panic: on Windows in particular,
        // `Instant` may not have an hour of monotonic-clock headroom this
        // early in a freshly started test process.
        cache.public_key("key-1").await.unwrap();
        {
            let mut guard = cache.cache.write().unwrap();
            let cached = guard.as_mut().unwrap();
            cached.ttl = Duration::ZERO;
        }

        cache.public_key("key-1").await.unwrap();
    }

    #[tokio::test]
    async fn malformed_response_body_surfaces_as_jwks_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let cache = cache_pointed_at(&server).await;
        let err = cache.public_key("key-1").await.unwrap_err();
        assert!(matches!(err, TokenVerificationError::Jwks(_)));
    }
}
