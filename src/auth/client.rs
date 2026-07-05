//! The `AuthClient` entry point and its builder.

use crate::auth::custom_token::CustomTokenSigner;
use crate::auth::error::AuthError;
use crate::auth::id_token::{IdTokenClaims, IdTokenVerifier, JwksCache};
use crate::auth::identity_toolkit::IdentityToolkitEndpoints;
use crate::auth::mode::ClientMode;
use crate::auth::session_cookie::{SessionCookieCertCache, SessionCookieVerifier};
use crate::auth::users::{
    CreateUserRequest, UpdateUserRequest, UserOperations, UserPage, UserRecord,
};
use crate::core::{Credentials, HttpClient, ProjectId, ServiceAccountKey};
use std::time::Duration;

/// Firebase Authentication client.
///
/// A single concrete type serves both live and emulator use: mode is a
/// runtime field ([`ClientMode`]), not a type parameter, so every method
/// below has exactly one signature regardless of environment. Build one with
/// [`AuthClientBuilder`].
pub struct AuthClient {
    http: HttpClient,
    project_id: ProjectId,
    mode: ClientMode,
    credentials: Credentials,
    id_token_verifier: IdTokenVerifier,
    session_cookie_verifier: SessionCookieVerifier,
    endpoints: IdentityToolkitEndpoints,
    #[cfg(feature = "live-user-management")]
    token_provider: tokio::sync::OnceCell<crate::auth::token_provider::TokenProvider>,
}

impl AuthClient {
    /// Starts building a new client for the given Firebase project.
    pub fn builder(project_id: impl Into<String>) -> AuthClientBuilder {
        AuthClientBuilder::new(project_id)
    }

    /// Returns the Firebase project id this client is configured for.
    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    /// Verifies a Firebase ID token, returning its claims.
    pub async fn verify_id_token(
        &self,
        token: &str,
    ) -> Result<crate::auth::id_token::IdTokenClaims, AuthError> {
        Ok(self.id_token_verifier.verify(token).await?)
    }

    /// Creates a Firebase custom token for the given uid.
    ///
    /// Requires the client to have been built with an explicit service
    /// account key; Application Default Credentials do not expose a
    /// private key and cannot sign custom tokens.
    pub fn create_custom_token(
        &self,
        uid: &str,
        claims: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String, AuthError> {
        let Credentials::ServiceAccount(key) = &self.credentials else {
            return Err(AuthError::Core(crate::core::CoreError::Credentials(
                "create_custom_token requires an explicit service account key".to_string(),
            )));
        };
        let signer = CustomTokenSigner::new((**key).clone());
        Ok(signer.create_custom_token(uid, claims)?)
    }

    /// Exchanges a verified ID token for a session cookie valid for
    /// `valid_duration` (Firebase allows up to 14 days).
    pub async fn create_session_cookie(
        &self,
        id_token: &str,
        valid_duration: Duration,
    ) -> Result<String, AuthError> {
        let token = self.bearer_token().await?;
        crate::auth::session_cookie::create_session_cookie(
            &self.http,
            &self.endpoints.create_session_cookie(),
            id_token,
            valid_duration,
            token.as_deref(),
            self.mode.emulator_api_key(),
        )
        .await
    }

    /// Verifies a Firebase session cookie, returning its claims.
    ///
    /// Verified against a different certificate endpoint and issuer than
    /// [`Self::verify_id_token`] — see [`crate::auth::session_cookie::verify`]
    /// for why the two aren't interchangeable.
    pub async fn verify_session_cookie(&self, cookie: &str) -> Result<IdTokenClaims, AuthError> {
        Ok(self.session_cookie_verifier.verify(cookie).await?)
    }

    /// Resolves an `Authorization: Bearer` token for calls to the Identity
    /// Toolkit REST API.
    ///
    /// In emulator mode, returns the literal string `"owner"` — the magic
    /// token the Firebase Auth Emulator recognizes as a privileged/admin
    /// caller (confirmed against the emulator's own source, which gates
    /// admin-only behavior in `accounts:signUp` and similar operations on
    /// whether the request carries recognized OAuth2 credentials; the
    /// official Admin SDKs send this same literal in emulator mode). Without
    /// it, admin calls like creating a user are instead treated as
    /// unprivileged client requests and rejected. In live mode, resolves a
    /// real OAuth2 access token from the configured credentials, requiring
    /// the `live-user-management` feature; without it, user-management calls
    /// in live mode fail with a clear error rather than silently sending an
    /// unauthenticated request.
    async fn bearer_token(&self) -> Result<Option<String>, AuthError> {
        if !self.mode.requires_bearer_token() {
            return Ok(self.mode.emulator_bearer_token().map(str::to_string));
        }

        #[cfg(feature = "live-user-management")]
        {
            let provider = self
                .token_provider
                .get_or_try_init(|| async {
                    match &self.credentials {
                        Credentials::ServiceAccount(key) => {
                            crate::auth::token_provider::TokenProvider::from_service_account(key)
                        }
                        Credentials::ApplicationDefault => {
                            crate::auth::token_provider::TokenProvider::from_application_default()
                                .await
                        }
                        Credentials::Emulator => unreachable!(
                            "requires_bearer_token() is false in emulator mode; \
                             bearer_token() returns before reaching this branch"
                        ),
                    }
                })
                .await?;
            return provider.access_token().await.map(Some);
        }

        #[cfg(not(feature = "live-user-management"))]
        {
            Err(AuthError::Core(crate::core::CoreError::Credentials(
                "user management against production Firebase requires the \
                 `live-user-management` feature"
                    .to_string(),
            )))
        }
    }

    fn user_operations<'a>(&'a self, bearer_token: Option<&'a str>) -> UserOperations<'a> {
        UserOperations::new(
            &self.http,
            &self.endpoints,
            bearer_token,
            self.mode.emulator_api_key(),
        )
    }

    /// Fetches a user by uid.
    pub async fn get_user(&self, uid: &str) -> Result<UserRecord, AuthError> {
        let token = self.bearer_token().await?;
        self.user_operations(token.as_deref()).get_user(uid).await
    }

    /// Fetches a user by email address.
    pub async fn get_user_by_email(&self, email: &str) -> Result<UserRecord, AuthError> {
        let token = self.bearer_token().await?;
        self.user_operations(token.as_deref())
            .get_user_by_email(email)
            .await
    }

    /// Creates a new user.
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<UserRecord, AuthError> {
        let token = self.bearer_token().await?;
        self.user_operations(token.as_deref())
            .create_user(request)
            .await
    }

    /// Updates an existing user.
    pub async fn update_user(
        &self,
        uid: &str,
        request: UpdateUserRequest,
    ) -> Result<UserRecord, AuthError> {
        let token = self.bearer_token().await?;
        self.user_operations(token.as_deref())
            .update_user(uid, request)
            .await
    }

    /// Replaces a user's custom claims.
    pub async fn set_custom_user_claims(
        &self,
        uid: &str,
        claims: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), AuthError> {
        let token = self.bearer_token().await?;
        self.user_operations(token.as_deref())
            .set_custom_user_claims(uid, claims)
            .await
    }

    /// Deletes a user by uid.
    pub async fn delete_user(&self, uid: &str) -> Result<(), AuthError> {
        let token = self.bearer_token().await?;
        self.user_operations(token.as_deref())
            .delete_user(uid)
            .await
    }

    /// Lists users, paginated via `next_page_token`.
    pub async fn list_users(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<UserPage, AuthError> {
        let token = self.bearer_token().await?;
        self.user_operations(token.as_deref())
            .list_users(max_results, page_token)
            .await
    }
}

/// Builds an [`AuthClient`].
pub struct AuthClientBuilder {
    project_id: String,
    service_account: Option<ServiceAccountKey>,
    #[cfg(feature = "live-user-management")]
    use_application_default_credentials: bool,
    emulator_host: Option<String>,
    http_client: Option<reqwest::Client>,
}

impl AuthClientBuilder {
    /// Starts building a client for the given Firebase project id.
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            service_account: None,
            #[cfg(feature = "live-user-management")]
            use_application_default_credentials: false,
            emulator_host: None,
            http_client: None,
        }
    }

    /// Authenticates using an explicit service account key.
    pub fn service_account_key(mut self, key: ServiceAccountKey) -> Self {
        self.service_account = Some(key);
        self
    }

    /// Authenticates using Application Default Credentials, resolved on
    /// first use: the `GOOGLE_APPLICATION_CREDENTIALS` environment
    /// variable, gcloud user credentials, or the GCE/Cloud Run metadata
    /// server, in that order (see [`gcp_auth::provider`]).
    #[cfg(feature = "live-user-management")]
    pub fn application_default_credentials(mut self) -> Self {
        self.use_application_default_credentials = true;
        self
    }

    /// Targets a Firebase Auth Emulator at `host` (e.g. `localhost:9099`)
    /// instead of production Firebase.
    ///
    /// If not called, the client still auto-detects the
    /// `FIREBASE_AUTH_EMULATOR_HOST` environment variable in [`Self::build`].
    #[cfg(feature = "emulator")]
    pub fn use_emulator(mut self, host: impl Into<String>) -> Self {
        self.emulator_host = Some(host.into());
        self
    }

    /// Supplies a custom [`reqwest::Client`], e.g. for testing.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Builds the [`AuthClient`].
    pub fn build(self) -> Result<AuthClient, AuthError> {
        let project_id = ProjectId::new(self.project_id)?;
        let mode = ClientMode::resolve(self.emulator_host);

        let credentials = if let Some(key) = self.service_account {
            Credentials::ServiceAccount(Box::new(key))
        } else {
            #[cfg(feature = "live-user-management")]
            if self.use_application_default_credentials {
                Credentials::ApplicationDefault
            } else if matches!(mode, ClientMode::Emulator { .. }) {
                Credentials::Emulator
            } else {
                return Err(AuthError::Core(crate::core::CoreError::Credentials(
                    "no credentials configured: call service_account_key(...) or \
                     application_default_credentials()"
                        .to_string(),
                )));
            }
            #[cfg(not(feature = "live-user-management"))]
            if matches!(mode, ClientMode::Emulator { .. }) {
                Credentials::Emulator
            } else {
                return Err(AuthError::Core(crate::core::CoreError::Credentials(
                    "no credentials configured: call service_account_key(...)".to_string(),
                )));
            }
        };

        let http = HttpClient::new(self.http_client.unwrap_or_default());
        let endpoints = mode.endpoints(project_id.as_str());
        let jwks = JwksCache::new(http.clone());
        let id_token_verifier = IdTokenVerifier::new(project_id.clone(), jwks);
        let session_cookie_certs = SessionCookieCertCache::new(http.clone());
        let session_cookie_verifier =
            SessionCookieVerifier::new(project_id.clone(), session_cookie_certs);

        Ok(AuthClient {
            http,
            project_id,
            mode,
            credentials,
            id_token_verifier,
            session_cookie_verifier,
            endpoints,
            #[cfg(feature = "live-user-management")]
            token_provider: tokio::sync::OnceCell::new(),
        })
    }
}
