//! Wire-format request/response DTOs for the Identity Toolkit REST API.
//!
//! These types mirror Google's JSON shapes exactly and are intentionally
//! kept separate from the ergonomic types in [`crate::auth::users::model`] so
//! that REST field-naming quirks (`localId`, `passwordHash`, ...) never leak
//! into the crate's public API.

use serde::{Deserialize, Serialize};

/// Request body for `accounts:lookup`.
#[derive(Debug, Default, Serialize)]
pub struct LookupRequest {
    #[serde(rename = "localId", skip_serializing_if = "Vec::is_empty")]
    pub local_id: Vec<String>,
    #[serde(rename = "email", skip_serializing_if = "Vec::is_empty")]
    pub email: Vec<String>,
}

/// A single user record as returned by the Identity Toolkit API.
#[derive(Debug, Clone, Deserialize)]
pub struct AccountInfo {
    #[serde(rename = "localId")]
    pub local_id: String,
    pub email: Option<String>,
    #[serde(rename = "emailVerified", default)]
    pub email_verified: bool,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub disabled: Option<bool>,
    #[serde(rename = "customAttributes")]
    pub custom_attributes: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "lastLoginAt")]
    pub last_login_at: Option<String>,
}

/// Response body for `accounts:lookup` and `accounts:batchGet`.
#[derive(Debug, Deserialize)]
pub struct AccountsResponse {
    #[serde(default)]
    pub users: Vec<AccountInfo>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

/// Request body for `accounts:signUp` (create user).
#[derive(Debug, Default, Serialize)]
pub struct SignUpRequest {
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "disabled", skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

/// Request body for `accounts:update`, including custom claims.
#[derive(Debug, Default, Serialize)]
pub struct UpdateRequest {
    #[serde(rename = "localId")]
    pub local_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "disableUser", skip_serializing_if = "Option::is_none")]
    pub disable_user: Option<bool>,
    #[serde(rename = "customAttributes", skip_serializing_if = "Option::is_none")]
    pub custom_attributes: Option<String>,
}

/// Request body for `accounts:delete`.
#[derive(Debug, Serialize)]
pub struct DeleteRequest {
    #[serde(rename = "localId")]
    pub local_id: String,
}
