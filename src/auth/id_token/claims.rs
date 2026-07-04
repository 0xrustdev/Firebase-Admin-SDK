//! Claim shapes for Firebase ID tokens.

use serde::{Deserialize, Serialize};

/// The decoded and verified claims of a Firebase ID token.
///
/// See <https://firebase.google.com/docs/auth/admin/verify-id-tokens> for the
/// authoritative claim reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
    /// The token issuer, expected to be `https://securetoken.google.com/<project-id>`.
    pub iss: String,
    /// The intended audience, expected to equal the Firebase project ID.
    pub aud: String,
    /// Issued-at time, in seconds since the Unix epoch.
    pub iat: i64,
    /// Expiration time, in seconds since the Unix epoch.
    pub exp: i64,
    /// The time the end user authenticated, in seconds since the Unix epoch.
    pub auth_time: i64,
    /// The Firebase user id (UID) this token was issued for.
    pub sub: String,
    /// The user's email address, if available.
    pub email: Option<String>,
    /// Whether the user's email address has been verified.
    #[serde(default)]
    pub email_verified: bool,
    /// Custom claims attached to the user via `set_custom_user_claims`.
    #[serde(flatten)]
    pub custom_claims: serde_json::Map<String, serde_json::Value>,
}
