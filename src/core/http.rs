//! Shared HTTP client wrapper used by all service modules.

use std::time::Duration;

/// Thin wrapper around a [`reqwest::Client`], giving service modules a single
/// seam for future cross-cutting concerns (retries, backoff, request tracing).
#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    /// Wraps an existing [`reqwest::Client`].
    pub fn new(inner: reqwest::Client) -> Self {
        Self { inner }
    }

    /// Returns the underlying [`reqwest::Client`].
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new(reqwest::Client::new())
    }
}

/// Parses the `max-age` directive out of a `Cache-Control` header value.
///
/// Shared by every public-key cache in the crate (ID token JWKS, session
/// cookie certs) so both respect Google's actual cache lifetime instead of
/// each hardcoding their own guess.
pub(crate) fn parse_cache_control_max_age(cache_control: &str) -> Option<Duration> {
    cache_control.split(',').find_map(|part| {
        let part = part.trim();
        let value = part.strip_prefix("max-age=")?;
        value.parse::<u64>().ok().map(Duration::from_secs)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_max_age_from_cache_control_header() {
        assert_eq!(
            parse_cache_control_max_age("public, max-age=21600, must-revalidate"),
            Some(Duration::from_secs(21600))
        );
        assert_eq!(parse_cache_control_max_age("no-store"), None);
        assert_eq!(parse_cache_control_max_age(""), None);
    }
}
