//! Firebase custom token creation.
//!
//! Custom tokens are signed locally with a service account's RSA private key
//! and require no network call. See
//! <https://firebase.google.com/docs/auth/admin/create-custom-tokens#create_custom_tokens_using_a_third-party_jwt_library>.

use crate::core::ServiceAccountKey;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CUSTOM_TOKEN_AUDIENCE: &str =
    "https://identitytoolkit.googleapis.com/google.identity.identitytoolkit.v1.IdentityToolkit";
const CUSTOM_TOKEN_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Serialize)]
struct CustomTokenClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: i64,
    exp: i64,
    uid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    claims: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Signs Firebase custom tokens using a service account's private key.
pub struct CustomTokenSigner {
    service_account: ServiceAccountKey,
}

impl CustomTokenSigner {
    /// Creates a signer that uses the given service account's key.
    pub fn new(service_account: ServiceAccountKey) -> Self {
        Self { service_account }
    }

    /// Creates a custom token for the given user id, optionally embedding
    /// additional custom claims.
    pub fn create_custom_token(
        &self,
        uid: &str,
        claims: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_secs() as i64;

        let payload = CustomTokenClaims {
            iss: self.service_account.client_email.clone(),
            sub: self.service_account.client_email.clone(),
            aud: CUSTOM_TOKEN_AUDIENCE.to_string(),
            iat: now,
            exp: now + CUSTOM_TOKEN_TTL.as_secs() as i64,
            uid: uid.to_string(),
            claims,
        };

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.service_account.private_key_id.clone());

        let encoding_key = EncodingKey::from_rsa_pem(self.service_account.private_key.as_bytes())?;

        encode(&header, &payload, &encoding_key)
    }
}
