//! Fetching and caching Google's X.509 signing certificates used to verify
//! Firebase session cookies.
//!
//! Session cookies are verified against a different key set than ID tokens:
//! `https://www.googleapis.com/identitytoolkit/v3/relyingparty/publicKeys`,
//! which returns a flat `{ "<kid>": "<PEM-encoded X.509 certificate>" }`
//! object rather than a JWK Set. This was confirmed by inspecting both the
//! official Node.js and Python Admin SDKs (`COOKIE_CERT_URI` /
//! `ID_TOKEN_CERT_URI` in `token-verifier.ts` / `_token_gen.py`) and by
//! fetching the endpoint directly, since a Firebase-Admin-SDK-agnostic RS256
//! JWK Set is *also* published for ID tokens at a different URL and it would
//! be easy to wrongly assume both token types share one key set.

use crate::auth::error::TokenVerificationError;
use crate::core::http::parse_cache_control_max_age;
use crate::core::HttpClient;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use x509_parser::prelude::{FromDer, X509Certificate};

const SESSION_COOKIE_CERTS_URL: &str =
    "https://www.googleapis.com/identitytoolkit/v3/relyingparty/publicKeys";

/// The minimum amount of time a cached certificate set is trusted before a
/// refresh is attempted, even if the response did not specify a
/// `Cache-Control` max-age.
const MIN_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

struct Cached {
    /// RSA public key components (`n`, `e`, base64url-encoded), extracted
    /// from each certificate's SubjectPublicKeyInfo, keyed by `kid`.
    keys: HashMap<String, (String, String)>,
    fetched_at: Instant,
    ttl: Duration,
}

/// Fetches and caches the X.509 certificates Google publishes for verifying
/// Firebase session cookies.
///
/// Mirrors [`crate::auth::id_token::JwksCache`]'s single-flight refresh
/// design: concurrent lookups that miss the cache at the same time share one
/// in-flight fetch rather than each issuing their own HTTP request.
pub struct SessionCookieCertCache {
    url: String,
    http: HttpClient,
    cache: RwLock<Option<Cached>>,
    refresh_lock: Mutex<()>,
}

impl SessionCookieCertCache {
    /// Creates a cache pointed at the standard session-cookie certs endpoint.
    pub fn new(http: HttpClient) -> Self {
        Self {
            url: SESSION_COOKIE_CERTS_URL.to_string(),
            http,
            cache: RwLock::new(None),
            refresh_lock: Mutex::new(()),
        }
    }

    /// Returns the RSA public key components (`n`, `e`, base64url-encoded)
    /// for the given key id, fetching or refreshing the cache as needed.
    pub async fn public_key(&self, kid: &str) -> Result<(String, String), TokenVerificationError> {
        if let Some(key) = self.cached_key(kid) {
            return Ok(key);
        }

        let _guard = self.refresh_lock.lock().await;

        if let Some(key) = self.cached_key(kid) {
            return Ok(key);
        }

        self.refresh().await?;

        self.cached_key(kid)
            .ok_or(TokenVerificationError::InvalidSignature)
    }

    fn cached_key(&self, kid: &str) -> Option<(String, String)> {
        let guard = self.cache.read().ok()?;
        let cached = guard.as_ref()?;
        if cached.fetched_at.elapsed() > cached.ttl {
            return None;
        }
        cached.keys.get(kid).cloned()
    }

    async fn refresh(&self) -> Result<(), TokenVerificationError> {
        let response = self
            .http
            .inner()
            .get(&self.url)
            .send()
            .await
            .map_err(|e| TokenVerificationError::Jwks(e.to_string()))?;

        let ttl = response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_cache_control_max_age)
            .unwrap_or(MIN_CACHE_TTL);

        let certs: HashMap<String, String> = response
            .json()
            .await
            .map_err(|e| TokenVerificationError::Jwks(e.to_string()))?;

        let mut keys = HashMap::with_capacity(certs.len());
        for (kid, pem) in certs {
            let (n, e) = rsa_components_from_certificate_pem(&pem)?;
            keys.insert(kid, (n, e));
        }

        let mut guard = self
            .cache
            .write()
            .map_err(|_| TokenVerificationError::Jwks("cert cache lock poisoned".to_string()))?;
        *guard = Some(Cached {
            keys,
            fetched_at: Instant::now(),
            ttl,
        });

        Ok(())
    }
}

/// Extracts the RSA public key modulus (`n`) and exponent (`e`), both
/// base64url-encoded, from a PEM-encoded X.509 certificate's
/// SubjectPublicKeyInfo.
///
/// Deliberately uses a real X.509 parser (`x509-parser`) rather than feeding
/// the certificate PEM directly to `jsonwebtoken::DecodingKey::from_rsa_pem`:
/// that function does not parse the X.509 `Certificate` structure at all —
/// it walks the raw DER for the first RSA/EC/Ed25519 OID it finds and treats
/// everything after it as key material, which happens to locate the right
/// bytes for typical certificates but isn't a structural guarantee. Parsing
/// the certificate properly and extracting `tbs_certificate.subject_pki`
/// removes that ambiguity for a security-critical verification path.
fn rsa_components_from_certificate_pem(
    pem: &str,
) -> Result<(String, String), TokenVerificationError> {
    use base64::Engine;

    let (_, pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes())
        .map_err(|e| TokenVerificationError::Jwks(format!("invalid certificate PEM: {e}")))?;
    let (_, cert) = X509Certificate::from_der(&pem.contents)
        .map_err(|e| TokenVerificationError::Jwks(format!("invalid certificate DER: {e}")))?;

    let spki = &cert.tbs_certificate.subject_pki;
    let public_key = spki
        .parsed()
        .map_err(|e| TokenVerificationError::Jwks(format!("unsupported public key: {e}")))?;

    let rsa_key = match public_key {
        x509_parser::public_key::PublicKey::RSA(rsa) => rsa,
        other => {
            return Err(TokenVerificationError::Jwks(format!(
                "expected an RSA public key, found {other:?}"
            )))
        }
    };

    let n = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(strip_leading_zero(rsa_key.modulus));
    let e = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(strip_leading_zero(rsa_key.exponent));
    Ok((n, e))
}

/// DER `INTEGER` encoding prepends a `0x00` byte when the most-significant
/// bit of the first "real" byte would otherwise be mistaken for a sign bit.
/// JWK's `n`/`e` fields are unsigned big-endian integers with no such
/// padding, so it must be stripped before base64url-encoding — leaving it in
/// would corrupt every RSA key whose modulus/exponent happens to have a
/// leading 1 bit (i.e. most real-world RSA moduli).
fn strip_leading_zero(bytes: &[u8]) -> &[u8] {
    match bytes {
        [0x00, rest @ ..] if !rest.is_empty() => rest,
        _ => bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const TEST_PRIVATE_KEY_PEM: &str = include_str!("../../../tests/fixtures/test_private_key.pem");
    /// A self-signed X.509 certificate generated from `TEST_PRIVATE_KEY_PEM`
    /// (`openssl req -new -x509 -key test_private_key.pem ...`), in the same
    /// PEM shape Google's session-cookie certs endpoint returns.
    const TEST_CERT_PEM: &str = include_str!("../../../tests/fixtures/test_cert.pem");

    #[test]
    fn extracts_the_correct_rsa_public_key_from_a_certificate() {
        let (n, e) = rsa_components_from_certificate_pem(TEST_CERT_PEM).unwrap();

        // Prove the extracted (n, e) are the *correct* key, not just
        // syntactically well-formed: sign a token with the certificate's
        // matching private key, then verify it using only the
        // freshly-extracted components. If `strip_leading_zero` or the SPKI
        // extraction were subtly wrong, this round trip would fail.
        let claims = json!({
            "sub": "someone",
            "iss": "issuer",
            "aud": "audience",
            "exp": 9_999_999_999i64,
        });
        let signing_key = EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY_PEM.as_bytes()).unwrap();
        let token = encode(&Header::new(Algorithm::RS256), &claims, &signing_key).unwrap();

        let decoding_key = DecodingKey::from_rsa_components(&n, &e).unwrap();
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&["audience"]);
        validation.set_issuer(&["issuer"]);
        decode::<serde_json::Value>(&token, &decoding_key, &validation)
            .expect("token should verify against the key extracted from the certificate");
    }

    #[test]
    fn rejects_a_malformed_certificate() {
        let err = rsa_components_from_certificate_pem("not a certificate").unwrap_err();
        assert!(matches!(err, TokenVerificationError::Jwks(_)));
    }

    #[tokio::test]
    async fn fetches_and_caches_certs_from_the_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/certs"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "test-kid": TEST_CERT_PEM })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let cache = SessionCookieCertCache {
            url: format!("{}/certs", server.uri()),
            http: HttpClient::default(),
            cache: RwLock::new(None),
            refresh_lock: Mutex::new(()),
        };

        let (n, e) = cache.public_key("test-kid").await.unwrap();
        let expected = rsa_components_from_certificate_pem(TEST_CERT_PEM).unwrap();
        assert_eq!((n, e), expected);

        // Second lookup should hit the cache, not the mock's `expect(1)`.
        cache.public_key("test-kid").await.unwrap();
    }
}
