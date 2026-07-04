//! Error types for the `auth` module.

use crate::core::CoreError;

/// Errors that can occur while verifying an ID token or session cookie.
#[derive(Debug, thiserror::Error)]
pub enum TokenVerificationError {
    /// The token's `exp` claim is in the past.
    #[error("token has expired")]
    Expired,
    /// The token's `iat`/`auth_time` claim is in the future.
    #[error("token is not yet valid")]
    NotYetValid,
    /// The token's signature did not verify against any known public key.
    #[error("token signature is invalid")]
    InvalidSignature,
    /// The token's `aud` claim did not match the configured project ID.
    #[error("token audience does not match project id")]
    AudienceMismatch,
    /// The token's `iss` claim did not match the expected issuer.
    #[error("token issuer is invalid")]
    IssuerMismatch,
    /// The token is missing a `sub` claim, or it is empty.
    #[error("token is missing a subject claim")]
    MissingSubject,
    /// The token could not be decoded or its header/claims could not be parsed.
    #[error("malformed token: {0}")]
    Malformed(#[from] jsonwebtoken::errors::Error),
    /// Google's public keys (JWKS) could not be fetched or parsed.
    #[error("failed to fetch signing keys: {0}")]
    Jwks(String),
}

/// The top-level error type for all `auth` module operations.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// A lower-level, service-independent error occurred.
    #[error(transparent)]
    Core(#[from] CoreError),

    /// The underlying HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// ID token or session cookie verification failed.
    #[error("token verification failed: {0}")]
    TokenVerification(#[from] TokenVerificationError),

    /// The Firebase Identity Toolkit API returned an error response.
    #[error("Firebase Auth API error ({status}): {message}")]
    Api {
        /// HTTP status code returned by the API.
        status: u16,
        /// Human-readable error message returned by the API.
        message: String,
        /// Machine-readable error code, when the API provides one.
        error_code: Option<String>,
    },

    /// Signing a custom token or session cookie failed.
    #[error("token signing failed: {0}")]
    Signing(#[from] jsonwebtoken::errors::Error),

    /// The requested user does not exist.
    #[error("user not found")]
    UserNotFound,
}
