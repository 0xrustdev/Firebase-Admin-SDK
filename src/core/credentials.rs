//! Service account credential loading.

use crate::core::error::CoreError;
use serde::Deserialize;

/// A parsed Firebase/GCP service account key file (JSON).
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountKey {
    /// The service account's client email address.
    pub client_email: String,
    /// The PEM-encoded RSA private key used to sign tokens.
    pub private_key: String,
    /// The GCP project ID this service account belongs to.
    pub project_id: String,
    /// Unique key identifier assigned by Google, used as the JWT `kid` header.
    pub private_key_id: String,
}

impl ServiceAccountKey {
    /// Parses a service account key from its raw JSON representation.
    pub fn from_json(json: &str) -> Result<Self, CoreError> {
        serde_json::from_str(json).map_err(CoreError::Deserialize)
    }

    /// Reads and parses a service account key from a file path.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, CoreError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| CoreError::Credentials(format!("failed to read key file: {e}")))?;
        Self::from_json(&contents)
    }
}

/// The source of credentials an [`crate::auth::AuthClientBuilder`] should use.
#[derive(Debug, Clone)]
pub enum Credentials {
    /// A explicitly-provided service account key.
    ServiceAccount(Box<ServiceAccountKey>),
    /// Application Default Credentials, resolved at request time.
    #[cfg(feature = "application-default-credentials")]
    ApplicationDefault,
    /// No credentials; only valid when talking to the Firebase emulator.
    Emulator,
}
