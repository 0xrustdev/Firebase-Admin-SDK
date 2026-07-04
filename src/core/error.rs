//! Error types shared across all Firebase service modules.

/// Errors that can occur in service-independent core operations
/// (HTTP transport, credential loading, project ID resolution).
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// The underlying HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// A response body could not be deserialized.
    #[error("failed to parse response: {0}")]
    Deserialize(#[from] serde_json::Error),

    /// Service account or application-default credentials could not be loaded.
    #[error("credential error: {0}")]
    Credentials(String),

    /// The configured Firebase/GCP project ID is missing or invalid.
    #[error("invalid project id: {0}")]
    InvalidProjectId(String),
}
