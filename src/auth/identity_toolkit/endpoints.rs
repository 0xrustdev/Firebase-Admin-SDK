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
}

impl IdentityToolkitEndpoints {
    /// Endpoints pointed at the production Identity Toolkit API.
    pub fn live() -> Self {
        Self {
            base: IDENTITY_TOOLKIT_BASE.to_string(),
        }
    }

    /// Endpoints pointed at a local Firebase Auth Emulator instance.
    ///
    /// `host` is the emulator host and port, e.g. `localhost:9099`.
    pub fn emulator(host: &str) -> Self {
        Self {
            base: format!("http://{host}/identitytoolkit.googleapis.com/v1"),
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

    /// `accounts:batchGet` — list users, paginated via `nextPageToken`.
    pub fn batch_get(&self) -> String {
        format!("{}/accounts:batchGet", self.base)
    }

    /// `accounts:createSessionCookie` — exchange an ID token for a session cookie.
    pub fn create_session_cookie(&self) -> String {
        format!("{}/accounts:createSessionCookie", self.base)
    }
}
