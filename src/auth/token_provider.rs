//! OAuth2 bearer token acquisition for calls to the Identity Toolkit REST API.
//!
//! Only compiled in when the `live-user-management` feature is enabled.
//! Wraps [`gcp_auth`], which handles both explicit service-account
//! credentials and Application Default Credentials through the same
//! [`gcp_auth::TokenProvider`] trait, and caches tokens internally until
//! they're close to expiry.

use crate::auth::error::AuthError;
use crate::core::{CoreError, ServiceAccountKey};
use std::sync::Arc;

/// The OAuth2 scope required to call the Identity Toolkit REST API.
///
/// See <https://cloud.google.com/identity-platform/docs/reference/rest> —
/// Identity Toolkit endpoints accept the general-purpose cloud-platform
/// scope.
const IDENTITY_TOOLKIT_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/// Resolves and caches OAuth2 bearer tokens for live-mode Identity Toolkit
/// calls.
pub struct TokenProvider {
    inner: Arc<dyn gcp_auth::TokenProvider>,
}

impl std::fmt::Debug for TokenProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenProvider").finish_non_exhaustive()
    }
}

impl TokenProvider {
    /// Builds a token provider backed by the given service account key.
    pub fn from_service_account(key: &ServiceAccountKey) -> Result<Self, AuthError> {
        let json = serde_json::to_string(&ServiceAccountKeyJson::from(key))
            .map_err(CoreError::Deserialize)?;
        let account = gcp_auth::CustomServiceAccount::from_json(&json).map_err(|e| {
            AuthError::Core(CoreError::Credentials(format!(
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
    pub async fn from_application_default() -> Result<Self, AuthError> {
        let inner = gcp_auth::provider().await.map_err(|e| {
            AuthError::Core(CoreError::Credentials(format!(
                "failed to resolve Application Default Credentials: {e}"
            )))
        })?;
        Ok(Self { inner })
    }

    /// Returns a valid OAuth2 access token, fetching or refreshing one as
    /// needed. Cached internally by `gcp_auth` until close to expiry.
    pub async fn access_token(&self) -> Result<String, AuthError> {
        let token = self
            .inner
            .token(&[IDENTITY_TOOLKIT_SCOPE])
            .await
            .map_err(|e| {
                AuthError::Core(CoreError::Credentials(format!(
                    "failed to acquire an OAuth2 access token: {e}"
                )))
            })?;
        Ok(token.as_str().to_string())
    }
}

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
}

impl<'a> From<&'a ServiceAccountKey> for ServiceAccountKeyJson<'a> {
    fn from(key: &'a ServiceAccountKey) -> Self {
        Self {
            r#type: "service_account",
            client_email: &key.client_email,
            private_key: &key.private_key,
            private_key_id: &key.private_key_id,
            project_id: &key.project_id,
        }
    }
}
