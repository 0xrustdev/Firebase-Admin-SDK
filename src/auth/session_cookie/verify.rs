//! Session cookie verification.
//!
//! Session cookies are RS256-signed JWTs like ID tokens, but with two
//! differences confirmed against the official Node.js and Python Admin SDK
//! source (`COOKIE_CERT_URI`/`COOKIE_ISSUER_PREFIX` vs.
//! `ID_TOKEN_CERT_URI`/`ID_TOKEN_ISSUER_PREFIX` in
//! `token-verifier.ts`/`_token_gen.py`), and directly against Google's
//! endpoints:
//!
//! - **Issuer**: `https://session.firebase.google.com/<project-id>`, not
//!   `https://securetoken.google.com/<project-id>`.
//! - **Signing keys**: a separate X.509 certificate endpoint
//!   (`https://www.googleapis.com/identitytoolkit/v3/relyingparty/publicKeys`),
//!   not the securetoken JWKS used for ID tokens — see
//!   [`crate::auth::session_cookie::certs`].

use crate::auth::error::TokenVerificationError;
use crate::auth::id_token::verifier::verify_with_key;
use crate::auth::id_token::IdTokenClaims;
use crate::auth::session_cookie::certs::SessionCookieCertCache;
use crate::core::ProjectId;
use jsonwebtoken::{decode_header, Algorithm, DecodingKey};

const SESSION_COOKIE_ISSUER_PREFIX: &str = "https://session.firebase.google.com/";

/// Verifies Firebase session cookies against Google's session-cookie
/// certificate endpoint and the project's expected issuer/audience.
pub struct SessionCookieVerifier {
    project_id: ProjectId,
    certs: SessionCookieCertCache,
}

impl SessionCookieVerifier {
    /// Creates a verifier for the given project, using the given cert cache.
    pub fn new(project_id: ProjectId, certs: SessionCookieCertCache) -> Self {
        Self { project_id, certs }
    }

    /// Verifies a session cookie, returning its claims if every check
    /// passes: signature, `exp`, `iat`/`auth_time`, `aud`, `iss`, and
    /// non-empty `sub`.
    pub async fn verify(&self, cookie: &str) -> Result<IdTokenClaims, TokenVerificationError> {
        let header = decode_header(cookie)?;

        if header.alg != Algorithm::RS256 {
            return Err(TokenVerificationError::InvalidSignature);
        }

        let kid = header.kid.ok_or(TokenVerificationError::InvalidSignature)?;
        let (n, e) = self.certs.public_key(&kid).await?;
        let decoding_key = DecodingKey::from_rsa_components(&n, &e)
            .map_err(|_| TokenVerificationError::InvalidSignature)?;

        verify_with_key(
            cookie,
            &decoding_key,
            self.project_id.as_str(),
            SESSION_COOKIE_ISSUER_PREFIX,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde_json::json;

    const TEST_PRIVATE_KEY_PEM: &str = include_str!("../../../tests/fixtures/test_private_key.pem");
    const TEST_PROJECT_ID: &str = "test-project";

    fn decoding_key() -> DecodingKey {
        DecodingKey::from_rsa_pem(include_bytes!(
            "../../../tests/fixtures/test_public_key.pem"
        ))
        .unwrap()
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn sign(claims: &serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key".to_string());
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY_PEM.as_bytes()).unwrap();
        encode(&header, claims, &key).unwrap()
    }

    fn valid_session_cookie_claims(now: i64) -> serde_json::Value {
        json!({
            "iss": format!("{SESSION_COOKIE_ISSUER_PREFIX}{TEST_PROJECT_ID}"),
            "aud": TEST_PROJECT_ID,
            "iat": now - 10,
            "exp": now + 3600,
            "auth_time": now - 10,
            "sub": "test-uid",
        })
    }

    #[test]
    fn accepts_a_valid_session_cookie() {
        let token = sign(&valid_session_cookie_claims(now()));
        let claims = verify_with_key(
            &token,
            &decoding_key(),
            TEST_PROJECT_ID,
            SESSION_COOKIE_ISSUER_PREFIX,
        )
        .unwrap();
        assert_eq!(claims.sub, "test-uid");
    }

    #[test]
    fn rejects_an_id_token_issuer_on_a_session_cookie_endpoint() {
        // A valid ID token (securetoken.google.com issuer) must NOT verify
        // as a session cookie — this is exactly the confusion the separate
        // issuer check exists to prevent.
        let mut claims = valid_session_cookie_claims(now());
        claims["iss"] = json!(format!("https://securetoken.google.com/{TEST_PROJECT_ID}"));
        let token = sign(&claims);

        let err = verify_with_key(
            &token,
            &decoding_key(),
            TEST_PROJECT_ID,
            SESSION_COOKIE_ISSUER_PREFIX,
        )
        .unwrap_err();
        assert!(matches!(err, TokenVerificationError::IssuerMismatch));
    }

    #[test]
    fn rejects_an_expired_session_cookie() {
        let mut claims = valid_session_cookie_claims(now());
        claims["exp"] = json!(now() - 100);
        let token = sign(&claims);

        let err = verify_with_key(
            &token,
            &decoding_key(),
            TEST_PROJECT_ID,
            SESSION_COOKIE_ISSUER_PREFIX,
        )
        .unwrap_err();
        assert!(matches!(err, TokenVerificationError::Expired));
    }
}
