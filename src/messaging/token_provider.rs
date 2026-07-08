//! OAuth2 bearer token acquisition for calls to the FCM v1 and Instance ID
//! REST APIs.
//!
//! Only compiled in when the `live-messaging` feature is enabled. Mirrors
//! `crate::auth::token_provider` — wraps [`gcp_auth`], which handles both
//! explicit service-account credentials and Application Default Credentials
//! through the same [`gcp_auth::TokenProvider`] trait, and caches tokens
//! internally until they're close to expiry.

use crate::core::{CoreError, ServiceAccountKey};
use crate::messaging::error::MessagingError;
use std::sync::Arc;

/// The OAuth2 scope required to call the FCM v1 and Instance ID REST APIs.
///
/// See <https://firebase.google.com/docs/cloud-messaging/auth-server> — both
/// accept the general-purpose cloud-platform scope.
const MESSAGING_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/// Resolves and caches OAuth2 bearer tokens for FCM v1 and Instance ID calls.
pub(crate) struct TokenProvider {
    inner: Arc<dyn gcp_auth::TokenProvider>,
}

impl std::fmt::Debug for TokenProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenProvider").finish_non_exhaustive()
    }
}

impl TokenProvider {
    /// Builds a token provider backed by the given service account key.
    pub(crate) fn from_service_account(key: &ServiceAccountKey) -> Result<Self, MessagingError> {
        let json = serde_json::to_string(&ServiceAccountKeyJson::from(key))
            .map_err(CoreError::Deserialize)?;
        let account = gcp_auth::CustomServiceAccount::from_json(&json).map_err(|e| {
            MessagingError::Core(CoreError::Credentials(format!(
                "failed to initialize service account credentials: {e}"
            )))
        })?;
        Ok(Self {
            inner: Arc::new(account),
        })
    }

    /// Builds a token provider that resolves Application Default
    /// Credentials at first use (`GOOGLE_APPLICATION_CREDENTIALS` env var,
    /// gcloud user credentials, or the GCE/Cloud Run metadata server, in
    /// that order — see [`gcp_auth::provider`]).
    pub(crate) async fn from_application_default() -> Result<Self, MessagingError> {
        let inner = gcp_auth::provider().await.map_err(|e| {
            MessagingError::Core(CoreError::Credentials(format!(
                "failed to resolve Application Default Credentials: {e}"
            )))
        })?;
        Ok(Self { inner })
    }

    /// Returns a valid OAuth2 access token, fetching or refreshing one as
    /// needed. Cached internally by `gcp_auth` until close to expiry.
    pub(crate) async fn access_token(&self) -> Result<String, MessagingError> {
        let token = self.inner.token(&[MESSAGING_SCOPE]).await.map_err(|e| {
            MessagingError::Core(CoreError::Credentials(format!(
                "failed to acquire an OAuth2 access token: {e}"
            )))
        })?;
        Ok(token.as_str().to_string())
    }
}

/// The OAuth2 token endpoint every standard Google service account key uses
/// — a fixed constant, not per-key data, but required by `gcp_auth`'s
/// internal deserialization target (`ServiceAccountKey` in `gcp_auth`'s
/// `types.rs`, which requires `token_uri` as a non-optional `String`). Real
/// service account key files always carry this same value in their
/// `token_uri` field; since [`crate::core::ServiceAccountKey`] doesn't store
/// it, it's reconstructed here rather than dropped.
const GOOGLE_OAUTH2_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";

/// Mirrors the JSON shape `gcp_auth::CustomServiceAccount::from_json` expects
/// (Google's standard service-account key file format), built from our own
/// [`ServiceAccountKey`] so we don't need to keep the original JSON string
/// around after parsing it.
#[derive(serde::Serialize)]
struct ServiceAccountKeyJson<'a> {
    r#type: &'static str,
    client_email: &'a str,
    private_key: &'a str,
    private_key_id: &'a str,
    project_id: &'a str,
    token_uri: &'static str,
}

impl<'a> From<&'a ServiceAccountKey> for ServiceAccountKeyJson<'a> {
    fn from(key: &'a ServiceAccountKey) -> Self {
        Self {
            r#type: "service_account",
            client_email: &key.client_email,
            private_key: &key.private_key,
            private_key_id: &key.private_key_id,
            project_id: &key.project_id,
            token_uri: GOOGLE_OAUTH2_TOKEN_URI,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PRIVATE_KEY_PEM: &str = include_str!("../../tests/fixtures/test_private_key.pem");

    /// Regression test for a bug where the JSON reconstructed from
    /// [`ServiceAccountKey`] omitted `token_uri`, which `gcp_auth`'s
    /// internal deserialization target requires as non-optional — every
    /// real service account key would fail here with "missing field
    /// `token_uri`" before a single network call was ever made.
    #[test]
    fn from_service_account_produces_json_gcp_auth_can_parse() {
        let key = ServiceAccountKey {
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            private_key: TEST_PRIVATE_KEY_PEM.to_string(),
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
        };

        TokenProvider::from_service_account(&key)
            .expect("a well-formed service account key must parse successfully");
    }
}
