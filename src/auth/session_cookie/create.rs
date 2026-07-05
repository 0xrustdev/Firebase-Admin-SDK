//! Session cookie creation via the Identity Toolkit `:createSessionCookie` endpoint.

use crate::auth::error::{parse_identity_toolkit_response, AuthError};
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
/// `bearer_token` must be `Some` in live mode (this endpoint, like the rest
/// of the Identity Toolkit API, requires OAuth2 authentication there) and is
/// ignored in emulator mode, where `emulator_api_key` must be `Some` instead
/// — see [`crate::auth::mode::ClientMode::emulator_api_key`].
pub async fn create_session_cookie(
    http: &HttpClient,
    endpoint: &str,
    id_token: &str,
    valid_duration: Duration,
    bearer_token: Option<&str>,
    emulator_api_key: Option<&str>,
) -> Result<String, AuthError> {
    let body = CreateSessionCookieRequest {
        id_token,
        valid_duration_secs: valid_duration.as_secs(),
    };

    let mut request = http.inner().post(endpoint);
    if let Some(token) = bearer_token {
        request = request.bearer_auth(token);
    }
    if let Some(key) = emulator_api_key {
        request = request.query(&[("key", key)]);
    }

    let response = request.json(&body).send().await?;
    let parsed: CreateSessionCookieResponse = parse_identity_toolkit_response(response).await?;

    Ok(parsed.session_cookie)
}
