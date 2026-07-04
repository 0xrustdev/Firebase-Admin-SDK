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

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
    use serde::Deserialize;

    const TEST_PRIVATE_KEY_PEM: &str = include_str!("../../../tests/fixtures/test_private_key.pem");
    const TEST_PUBLIC_KEY_PEM: &[u8] =
        include_bytes!("../../../tests/fixtures/test_public_key.pem");

    #[derive(Debug, Deserialize)]
    struct DecodedClaims {
        iss: String,
        sub: String,
        aud: String,
        iat: i64,
        exp: i64,
        uid: String,
        claims: Option<serde_json::Map<String, serde_json::Value>>,
    }

    fn test_service_account() -> ServiceAccountKey {
        ServiceAccountKey {
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            private_key: TEST_PRIVATE_KEY_PEM.to_string(),
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
        }
    }

    /// Decodes independently of [`CustomTokenSigner`]/the crate's own
    /// verifier, using raw `jsonwebtoken` calls against the fixture public
    /// key, so this test can't pass merely because signing and verifying
    /// share a bug.
    fn independently_decode(token: &str) -> DecodedClaims {
        let header = decode_header(token).unwrap();
        assert_eq!(header.alg, Algorithm::RS256);
        assert_eq!(header.kid.as_deref(), Some("test-key-id"));

        let decoding_key = DecodingKey::from_rsa_pem(TEST_PUBLIC_KEY_PEM).unwrap();
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[CUSTOM_TOKEN_AUDIENCE]);
        validation.set_issuer(&["test@test-project.iam.gserviceaccount.com"]);
        validation.validate_exp = false; // signer sets exp ~1h out; nothing to validate against here.

        decode::<DecodedClaims>(token, &decoding_key, &validation)
            .unwrap()
            .claims
    }

    #[test]
    fn creates_a_token_with_the_documented_claim_shape() {
        let signer = CustomTokenSigner::new(test_service_account());
        let token = signer.create_custom_token("some-uid", None).unwrap();

        let claims = independently_decode(&token);
        assert_eq!(claims.uid, "some-uid");
        assert_eq!(claims.iss, "test@test-project.iam.gserviceaccount.com");
        assert_eq!(claims.sub, "test@test-project.iam.gserviceaccount.com");
        assert_eq!(claims.aud, CUSTOM_TOKEN_AUDIENCE);
        assert!(claims.exp > claims.iat);
        assert!(claims.exp - claims.iat <= CUSTOM_TOKEN_TTL.as_secs() as i64);
        assert!(claims.claims.is_none());
    }

    #[test]
    fn embeds_custom_claims_when_provided() {
        let signer = CustomTokenSigner::new(test_service_account());
        let mut extra = serde_json::Map::new();
        extra.insert("admin".to_string(), serde_json::Value::Bool(true));

        let token = signer
            .create_custom_token("some-uid", Some(extra.clone()))
            .unwrap();

        let claims = independently_decode(&token);
        assert_eq!(claims.claims, Some(extra));
    }
}
