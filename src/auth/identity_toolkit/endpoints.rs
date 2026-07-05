//! URL builders for the Identity Toolkit REST API.
//!
//! # Implementation status
//!
//! Paths below match the Identity Toolkit v1 REST reference as of this
//! writing, but must be re-confirmed against
//! <https://cloud.google.com/identity-platform/docs/reference/rest> before
//! the corresponding operations in [`crate::auth::users::operations`] are
//! implemented — do not assume they are exact.

const IDENTITY_TOOLKIT_BASE: &str = "https://identitytoolkit.googleapis.com/v1";

/// Builds Identity Toolkit v1 REST endpoint URLs for a given project.
pub struct IdentityToolkitEndpoints {
    base: String,
    project_id: String,
}

impl IdentityToolkitEndpoints {
    /// Endpoints pointed at the production Identity Toolkit API.
    pub fn live(project_id: &str) -> Self {
        Self {
            base: IDENTITY_TOOLKIT_BASE.to_string(),
            project_id: project_id.to_string(),
        }
    }

    /// Endpoints pointed at a local Firebase Auth Emulator instance.
    ///
    /// `host` is the emulator host and port, e.g. `localhost:9099`.
    pub fn emulator(host: &str, project_id: &str) -> Self {
        Self {
            base: format!("http://{host}/identitytoolkit.googleapis.com/v1"),
            project_id: project_id.to_string(),
        }
    }

    /// Endpoints pointed at an arbitrary base URL.
    ///
    /// Used by tests to point at a mock HTTP server; not exposed outside the
    /// crate since real callers should use [`Self::live`] or
    /// [`Self::emulator`].
    #[cfg(test)]
    pub(crate) fn custom(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            project_id: "test-project".to_string(),
        }
    }

    /// `accounts:lookup` — fetch one or more users by uid, email, or phone number.
    pub fn lookup(&self) -> String {
        format!("{}/accounts:lookup", self.base)
    }

    /// `accounts:signUp` — create a new user (or, unauthenticated, sign one up).
    pub fn sign_up(&self) -> String {
        format!("{}/accounts:signUp", self.base)
    }

    /// `accounts:update` — update an existing user, including custom claims.
    pub fn update(&self) -> String {
        format!("{}/accounts:update", self.base)
    }

    /// `accounts:delete` — delete a user.
    pub fn delete(&self) -> String {
        format!("{}/accounts:delete", self.base)
    }

    /// `projects/{projectId}/accounts:batchGet` — list users, paginated via
    /// `nextPageToken`.
    ///
    /// Unlike the other `accounts:*` operations, `batchGet` requires the
    /// `projects/{projectId}` path segment and is called with `GET` (query
    /// parameters), not `POST` with a JSON body — confirmed against the
    /// Firebase Auth Emulator's own API spec after the flat
    /// `/accounts:batchGet` path (matching every other operation here)
    /// returned a 404 in practice.
    pub fn batch_get(&self) -> String {
        format!(
            "{}/projects/{}/accounts:batchGet",
            self.base, self.project_id
        )
    }

    /// `projects/{projectId}:createSessionCookie` — exchange an ID token for
    /// a session cookie.
    ///
    /// Not an `accounts:*` operation like the others in this crate — it's
    /// `projects/{projectId}:createSessionCookie` with the colon directly
    /// after the project ID segment. Confirmed against the Firebase Auth
    /// Emulator's own API spec after the previous `/accounts:createSessionCookie`
    /// path returned a 404 for every request, valid or not, in practice.
    pub fn create_session_cookie(&self) -> String {
        format!(
            "{}/projects/{}:createSessionCookie",
            self.base, self.project_id
        )
    }
}
