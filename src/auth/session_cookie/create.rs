//! Session cookie creation via the Identity Toolkit `:createSessionCookie` endpoint.

use crate::auth::error::AuthError;
use crate::core::HttpClient;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct CreateSessionCookieRequest<'a> {
    #[serde(rename = "idToken")]
    id_token: &'a str,
    #[serde(rename = "validDuration")]
    valid_duration_secs: u64,
}

#[derive(Debug, Deserialize)]
struct CreateSessionCookieResponse {
    #[serde(rename = "sessionCookie")]
    session_cookie: String,
}

/// Exchanges a verified ID token for a long-lived session cookie.
///
/// `endpoint` is the fully-qualified `:createSessionCookie` URL, which
/// differs between live and emulator [`crate::auth::mode::ClientMode`].
pub async fn create_session_cookie(
    http: &HttpClient,
    endpoint: &str,
    id_token: &str,
    valid_duration: Duration,
) -> Result<String, AuthError> {
    let body = CreateSessionCookieRequest {
        id_token,
        valid_duration_secs: valid_duration.as_secs(),
    };

    let response = http.inner().post(endpoint).json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(AuthError::Api {
            status,
            message,
            error_code: None,
        });
    }

    let parsed: CreateSessionCookieResponse = response
        .json()
        .await
        .map_err(|e| AuthError::Core(crate::core::CoreError::Http(e)))?;

    Ok(parsed.session_cookie)
}
