//! Error types for the `auth` module.

use crate::core::CoreError;
use serde::Deserialize;

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

/// The `{"error": {...}}` envelope Google APIs use for error responses.
///
/// See <https://cloud.google.com/apis/design/errors#http_mapping>. The
/// Identity Toolkit API additionally nests a well-known short code (e.g.
/// `EMAIL_EXISTS`, `USER_NOT_FOUND`, `WEAK_PASSWORD`) as the `message` field
/// of the first entry in `errors`, or as a suffix on the top-level `message`
/// (`"INVALID_ID_TOKEN : Firebase ID token has ..."`); both shapes appear in
/// the wild depending on the endpoint, so both are checked.
#[derive(Debug, Deserialize)]
struct GoogleApiErrorBody {
    error: GoogleApiError,
}

#[derive(Debug, Deserialize)]
struct GoogleApiError {
    message: String,
    #[serde(default)]
    errors: Vec<GoogleApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct GoogleApiErrorDetail {
    reason: Option<String>,
}

/// Turns a [`reqwest::Response`] into a parsed value, or an
/// [`AuthError::Api`]/[`AuthError::Core`] if the request failed or the
/// success body couldn't be deserialized.
///
/// Centralizes the "check status, read body, extract Identity Toolkit's
/// error code" logic shared by every Identity Toolkit call site (session
/// cookie creation, user management), so error-code parsing only needs to be
/// correct in one place.
pub(crate) async fn parse_identity_toolkit_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, AuthError> {
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no response body>".to_string());
        return Err(AuthError::from_api_response(status, &body));
    }

    response
        .json::<T>()
        .await
        .map_err(|e| AuthError::Core(CoreError::Http(e)))
}

impl AuthError {
    /// Builds an [`AuthError::Api`] from a non-success HTTP response,
    /// extracting Identity Toolkit's well-known short error code from the
    /// response body into `error_code` when present.
    fn from_api_response(status: u16, body: &str) -> Self {
        let (message, error_code) = match serde_json::from_str::<GoogleApiErrorBody>(body) {
            Ok(parsed) => {
                let code = parsed
                    .error
                    .errors
                    .first()
                    .and_then(|detail| detail.reason.clone())
                    .or_else(|| {
                        // Identity Toolkit often puts the short code as the
                        // whole message, or as a "CODE : detail" prefix.
                        parsed
                            .error
                            .message
                            .split(':')
                            .next()
                            .map(str::trim)
                            .filter(|s| {
                                !s.is_empty()
                                    && s.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                            })
                            .map(str::to_string)
                    });
                (parsed.error.message, code)
            }
            Err(_) => (body.to_string(), None),
        };

        AuthError::Api {
            status,
            message,
            error_code,
        }
    }
}
