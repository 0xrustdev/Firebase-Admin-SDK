//! Fetching and caching Google's public signing keys (JWKS) for ID token
//! verification.
//!
//! Keys are published at
//! `https://www.googleapis.com/service_accounts/v1/metadata/jwk/securetoken@system.gserviceaccount.com`.

use crate::auth::error::TokenVerificationError;
use crate::core::HttpClient;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

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
pub struct JwksCache {
    url: String,
    http: HttpClient,
    cache: RwLock<Option<Cached>>,
}

impl JwksCache {
    /// Creates a cache pointed at the standard securetoken JWKS endpoint.
    pub fn new(http: HttpClient) -> Self {
        Self {
            url: SECURETOKEN_JWKS_URL.to_string(),
            http,
            cache: RwLock::new(None),
        }
    }

    /// Returns the RSA public key components (`n`, `e`, base64url-encoded)
    /// for the given key id, fetching or refreshing the cache as needed.
    pub async fn public_key(&self, kid: &str) -> Result<(String, String), TokenVerificationError> {
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
            .and_then(parse_max_age)
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

fn parse_max_age(cache_control: &str) -> Option<Duration> {
    cache_control.split(',').find_map(|part| {
        let part = part.trim();
        let value = part.strip_prefix("max-age=")?;
        value.parse::<u64>().ok().map(Duration::from_secs)
    })
}
