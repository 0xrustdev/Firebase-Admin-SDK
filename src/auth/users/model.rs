//! Ergonomic, Rust-facing user types.
//!
//! These are distinct from the wire-format DTOs in
//! [`crate::auth::identity_toolkit::requests`] so that the public API never
//! exposes Google's REST field naming or shapes directly.

use crate::auth::identity_toolkit::requests::AccountInfo;
use serde_json::Map;

/// A Firebase user record.
#[derive(Debug, Clone)]
pub struct UserRecord {
    /// The user's unique Firebase id.
    pub uid: String,
    /// The user's email address, if set.
    pub email: Option<String>,
    /// Whether the user's email address has been verified.
    pub email_verified: bool,
    /// The user's display name, if set.
    pub display_name: Option<String>,
    /// Whether the user account is disabled.
    pub disabled: bool,
    /// Custom claims attached to this user via [`crate::auth::AuthClient`]
    /// custom-claims operations.
    pub custom_claims: Map<String, serde_json::Value>,
}

impl From<AccountInfo> for UserRecord {
    fn from(info: AccountInfo) -> Self {
        let custom_claims = info
            .custom_attributes
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();

        Self {
            uid: info.local_id,
            email: info.email,
            email_verified: info.email_verified,
            display_name: info.display_name,
            disabled: info.disabled.unwrap_or(false),
            custom_claims,
        }
    }
}

/// Fields accepted when creating a new user.
#[derive(Debug, Default, Clone)]
pub struct CreateUserRequest {
    /// An explicit uid for the new user; if omitted, Firebase assigns one.
    pub uid: Option<String>,
    /// The new user's email address.
    pub email: Option<String>,
    /// The new user's initial password.
    pub password: Option<String>,
    /// The new user's display name.
    pub display_name: Option<String>,
    /// Whether the new user account should start disabled.
    pub disabled: Option<bool>,
}

/// Fields accepted when updating an existing user. `None` fields are left
/// unchanged.
#[derive(Debug, Default, Clone)]
pub struct UpdateUserRequest {
    /// New email address.
    pub email: Option<String>,
    /// New display name.
    pub display_name: Option<String>,
    /// New disabled state.
    pub disabled: Option<bool>,
    /// Replaces the user's custom claims entirely, when set.
    pub custom_claims: Option<Map<String, serde_json::Value>>,
}
