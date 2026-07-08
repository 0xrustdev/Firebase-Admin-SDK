//! Error types for the `messaging` module.

use crate::core::CoreError;
use serde::Deserialize;

/// The top-level error type for all `messaging` module operations.
#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    /// A lower-level, service-independent error occurred.
    #[error(transparent)]
    Core(#[from] CoreError),

    /// The underlying HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The FCM v1 API returned an error response.
    #[error("FCM API error ({status}): {message}")]
    Api {
        /// HTTP status code returned by the API.
        status: u16,
        /// Human-readable error message returned by the API.
        message: String,
        /// Machine-readable error code, when the API provides one (e.g.
        /// `UNREGISTERED`, `INVALID_ARGUMENT`, `QUOTA_EXCEEDED`).
        error_code: Option<String>,
    },

    /// More than [`crate::messaging::MAX_BATCH_SIZE`] messages/tokens were
    /// passed to a batch operation
    /// ([`crate::messaging::MessagingClient::send_each`] or
    /// [`crate::messaging::MessagingClient::send_each_for_multicast`]).
    #[error("batch size {actual} exceeds the maximum of {max} messages/tokens per call")]
    BatchTooLarge {
        /// The number of messages/tokens that were passed in.
        actual: usize,
        /// The maximum allowed per call.
        max: usize,
    },
}

/// The `{"error": {...}}` envelope Google APIs use for error responses.
///
/// See <https://cloud.google.com/apis/design/errors#http_mapping>. FCM v1
/// additionally nests a well-known short status string (e.g. `UNREGISTERED`,
/// `SENDER_ID_MISMATCH`, `QUOTA_EXCEEDED`) inside `error.details`, as an
/// object whose `@type` ends in `FcmError` and carries an `errorCode` field.
/// Some error responses (e.g. auth/permission failures) omit that `details`
/// entry entirely; those still carry a top-level `error.status` gRPC
/// canonical code (`NOT_FOUND`, `PERMISSION_DENIED`, ...), which the official
/// Admin SDKs fall back to (`getErrorCode` in `messaging-errors-internal.ts`)
/// — mirrored here in [`MessagingError::from_api_response`].
#[derive(Debug, Deserialize)]
struct GoogleApiErrorBody {
    error: GoogleApiError,
}

#[derive(Debug, Deserialize)]
struct GoogleApiError {
    message: String,
    /// The gRPC canonical status string, e.g. `NOT_FOUND`,
    /// `INVALID_ARGUMENT`, `PERMISSION_DENIED`.
    status: Option<String>,
    #[serde(default)]
    details: Vec<GoogleApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct GoogleApiErrorDetail {
    #[serde(rename = "errorCode")]
    error_code: Option<String>,
}

/// Turns a [`reqwest::Response`] into a parsed value, or a
/// [`MessagingError::Api`]/[`MessagingError::Core`] if the request failed or
/// the success body couldn't be deserialized.
///
/// Centralizes the "check status, read body, extract FCM's error code" logic
/// shared by every FCM v1 call site, so error-code parsing only needs to be
/// correct in one place — mirrors
/// `crate::auth::error::parse_identity_toolkit_response`.
pub(crate) async fn parse_fcm_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, MessagingError> {
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no response body>".to_string());
        return Err(MessagingError::from_api_response(status, &body));
    }

    response
        .json::<T>()
        .await
        .map_err(|e| MessagingError::Core(CoreError::Http(e)))
}

impl MessagingError {
    /// Builds a [`MessagingError::Api`] from a non-success HTTP response,
    /// extracting FCM's well-known short error code from the response
    /// body. Prefers the FCM-specific `error.details[].errorCode` (e.g.
    /// `UNREGISTERED`); when that's absent, falls back to the gRPC
    /// canonical `error.status` (e.g. `NOT_FOUND`), matching the official
    /// Admin SDKs' fallback order.
    fn from_api_response(status: u16, body: &str) -> Self {
        let (message, error_code) = match serde_json::from_str::<GoogleApiErrorBody>(body) {
            Ok(parsed) => {
                let code = parsed
                    .error
                    .details
                    .iter()
                    .find_map(|d| d.error_code.clone())
                    .or_else(|| parsed.error.status.clone());
                (parsed.error.message, code)
            }
            Err(_) => (body.to_string(), None),
        };

        MessagingError::Api {
            status,
            message,
            error_code,
        }
    }
}
