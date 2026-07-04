//! Firebase ID token verification.
//!
//! See <https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library>
//! for the checks a compliant verifier must perform.

use crate::auth::error::TokenVerificationError;
use crate::auth::id_token::claims::IdTokenClaims;
use crate::auth::id_token::jwks::JwksCache;
use crate::core::ProjectId;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

/// Verifies Firebase ID tokens against Google's public keys and the
/// project's expected issuer/audience.
pub struct IdTokenVerifier {
    project_id: ProjectId,
    jwks: JwksCache,
}

impl IdTokenVerifier {
    /// Creates a verifier for the given project, using the given JWKS cache.
    pub fn new(project_id: ProjectId, jwks: JwksCache) -> Self {
        Self { project_id, jwks }
    }

    /// Verifies an ID token, returning its claims if every check passes:
    /// signature, `exp`, `iat`/`auth_time`, `aud`, `iss`, and non-empty `sub`.
    pub async fn verify(&self, token: &str) -> Result<IdTokenClaims, TokenVerificationError> {
        let header = decode_header(token)?;

        if header.alg != Algorithm::RS256 {
            return Err(TokenVerificationError::InvalidSignature);
        }

        let kid = header.kid.ok_or(TokenVerificationError::InvalidSignature)?;
        let (n, e) = self.jwks.public_key(&kid).await?;
        let decoding_key = DecodingKey::from_rsa_components(&n, &e)
            .map_err(|_| TokenVerificationError::InvalidSignature)?;

        verify_with_key(token, &decoding_key, self.project_id.as_str())
    }
}

/// Verifies a token's signature and claims against an already-resolved
/// [`DecodingKey`], independent of how that key was obtained. Split out from
/// [`IdTokenVerifier::verify`] so unit tests can exercise claim-validation
/// logic with fixture keys, without needing a live JWKS fetch.
pub(crate) fn verify_with_key(
    token: &str,
    decoding_key: &DecodingKey,
    project_id: &str,
) -> Result<IdTokenClaims, TokenVerificationError> {
    let expected_issuer = format!("https://securetoken.google.com/{project_id}");
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[project_id]);
    validation.set_issuer(&[expected_issuer]);
    validation.validate_exp = true;

    let token_data =
        decode::<IdTokenClaims>(token, decoding_key, &validation).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => TokenVerificationError::Expired,
            jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                TokenVerificationError::AudienceMismatch
            }
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                TokenVerificationError::IssuerMismatch
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                TokenVerificationError::InvalidSignature
            }
            _ => TokenVerificationError::Malformed(e),
        })?;

    let claims = token_data.claims;

    if claims.sub.trim().is_empty() {
        return Err(TokenVerificationError::MissingSubject);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_secs() as i64;

    if claims.auth_time > now {
        return Err(TokenVerificationError::NotYetValid);
    }

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    const TEST_PRIVATE_KEY_PEM: &str = include_str!("../../../tests/fixtures/test_private_key.pem");
    const TEST_PROJECT_ID: &str = "test-project";

    fn sign(claims: &serde_json::Value, alg: Algorithm) -> String {
        let mut header = Header::new(alg);
        header.kid = Some("test-key".to_string());
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY_PEM.as_bytes()).unwrap();
        encode(&header, claims, &key).unwrap()
    }

    fn decoding_key() -> DecodingKey {
        DecodingKey::from_rsa_pem(include_bytes!(
            "../../../tests/fixtures/test_public_key.pem"
        ))
        .unwrap()
    }

    fn valid_claims(now: i64) -> serde_json::Value {
        json!({
            "iss": format!("https://securetoken.google.com/{TEST_PROJECT_ID}"),
            "aud": TEST_PROJECT_ID,
            "iat": now - 10,
            "exp": now + 3600,
            "auth_time": now - 10,
            "sub": "test-uid",
        })
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn accepts_a_valid_token() {
        let token = sign(&valid_claims(now()), Algorithm::RS256);
        let claims = verify_with_key(&token, &decoding_key(), TEST_PROJECT_ID).unwrap();
        assert_eq!(claims.sub, "test-uid");
    }

    #[test]
    fn rejects_expired_token() {
        let mut claims = valid_claims(now());
        claims["exp"] = json!(now() - 100);
        let token = sign(&claims, Algorithm::RS256);
        let err = verify_with_key(&token, &decoding_key(), TEST_PROJECT_ID).unwrap_err();
        assert!(matches!(err, TokenVerificationError::Expired));
    }

    #[test]
    fn rejects_wrong_audience() {
        let mut claims = valid_claims(now());
        claims["aud"] = json!("some-other-project");
        let token = sign(&claims, Algorithm::RS256);
        let err = verify_with_key(&token, &decoding_key(), TEST_PROJECT_ID).unwrap_err();
        assert!(matches!(err, TokenVerificationError::AudienceMismatch));
    }

    #[test]
    fn rejects_wrong_issuer() {
        let mut claims = valid_claims(now());
        claims["iss"] = json!("https://securetoken.google.com/some-other-project");
        let token = sign(&claims, Algorithm::RS256);
        let err = verify_with_key(&token, &decoding_key(), TEST_PROJECT_ID).unwrap_err();
        assert!(matches!(err, TokenVerificationError::IssuerMismatch));
    }

    #[test]
    fn rejects_missing_subject() {
        let mut claims = valid_claims(now());
        claims["sub"] = json!("");
        let token = sign(&claims, Algorithm::RS256);
        let err = verify_with_key(&token, &decoding_key(), TEST_PROJECT_ID).unwrap_err();
        assert!(matches!(err, TokenVerificationError::MissingSubject));
    }

    #[test]
    fn rejects_not_yet_valid_auth_time() {
        let mut claims = valid_claims(now());
        claims["auth_time"] = json!(now() + 3600);
        let token = sign(&claims, Algorithm::RS256);
        let err = verify_with_key(&token, &decoding_key(), TEST_PROJECT_ID).unwrap_err();
        assert!(matches!(err, TokenVerificationError::NotYetValid));
    }

    #[test]
    fn rejects_tampered_signature() {
        let token = sign(&valid_claims(now()), Algorithm::RS256);
        let mut parts: Vec<&str> = token.split('.').collect();
        let tampered_payload = parts[1]
            .chars()
            .map(|c| if c == 'A' { 'B' } else { 'A' })
            .collect::<String>();
        parts[1] = &tampered_payload;
        let tampered = parts.join(".");
        let err = verify_with_key(&tampered, &decoding_key(), TEST_PROJECT_ID).unwrap_err();
        assert!(matches!(
            err,
            TokenVerificationError::InvalidSignature | TokenVerificationError::Malformed(_)
        ));
    }
}
