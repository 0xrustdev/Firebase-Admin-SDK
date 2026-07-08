//! Service account credential loading.

use crate::core::error::CoreError;
use serde::Deserialize;

/// A parsed Firebase/GCP service account key file (JSON).
///
/// The `Debug` implementation deliberately redacts [`Self::private_key`] so
/// that logging or panicking with a `ServiceAccountKey` in scope cannot leak
/// the private key into logs, error reports, or terminal output.
#[derive(Clone, Deserialize)]
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

impl std::fmt::Debug for ServiceAccountKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceAccountKey")
            .field("client_email", &self.client_email)
            .field("private_key", &"[redacted]")
            .field("project_id", &self.project_id)
            .field("private_key_id", &self.private_key_id)
            .finish()
    }
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
    #[cfg(any(feature = "live-user-management", feature = "live-messaging"))]
    ApplicationDefault,
    /// No credentials; only valid when talking to the Firebase emulator.
    Emulator,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_KEY_JSON: &str = r#"{
        "client_email": "test@test-project.iam.gserviceaccount.com",
        "private_key": "-----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY-----\n",
        "project_id": "test-project",
        "private_key_id": "abc123"
    }"#;

    #[test]
    fn parses_a_valid_key_from_json() {
        let key = ServiceAccountKey::from_json(VALID_KEY_JSON).unwrap();
        assert_eq!(
            key.client_email,
            "test@test-project.iam.gserviceaccount.com"
        );
        assert_eq!(key.project_id, "test-project");
        assert_eq!(key.private_key_id, "abc123");
    }

    #[test]
    fn rejects_malformed_json() {
        let err = ServiceAccountKey::from_json("not json").unwrap_err();
        assert!(matches!(err, CoreError::Deserialize(_)));
    }

    #[test]
    fn rejects_json_missing_required_fields() {
        let err = ServiceAccountKey::from_json(r#"{"client_email": "only-this@example.com"}"#)
            .unwrap_err();
        assert!(matches!(err, CoreError::Deserialize(_)));
    }

    #[test]
    fn from_file_surfaces_a_credentials_error_for_a_missing_path() {
        let err = ServiceAccountKey::from_file("/does/not/exist.json").unwrap_err();
        assert!(matches!(err, CoreError::Credentials(_)));
    }

    #[test]
    fn debug_output_redacts_the_private_key() {
        let key = ServiceAccountKey::from_json(VALID_KEY_JSON).unwrap();
        let debug_output = format!("{key:?}");
        assert!(!debug_output.contains("BEGIN PRIVATE KEY"));
        assert!(debug_output.contains("[redacted]"));
        assert!(debug_output.contains("test@test-project.iam.gserviceaccount.com"));
    }
}
