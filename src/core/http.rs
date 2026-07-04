//! Shared HTTP client wrapper used by all service modules.

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
