//! Session cookie verification.
//!
//! # Implementation status
//!
//! **Not yet implemented.** Firebase session cookies may be verified against
//! a *different* certificate endpoint than ID tokens
//! (historically `https://www.googleapis.com/identitytoolkit/v3/relyingparty/publicKeys`
//! for session cookies, versus the securetoken JWKS used by
//! [`crate::auth::id_token`]). This must be confirmed against current Firebase
//! documentation before implementing verification here — do not assume the
//! two share a key set.
//!
//! Tracked for v0.2.0. See the project roadmap in `ARCHITECTURE.md`.

use crate::auth::error::TokenVerificationError;

/// Verifies a Firebase session cookie and returns its claims.
///
/// # Panics
///
/// Always panics with `unimplemented!` in this release; see the module docs.
pub async fn verify_session_cookie(
    _cookie: &str,
) -> Result<crate::auth::id_token::IdTokenClaims, TokenVerificationError> {
    unimplemented!(
        "session cookie verification is not yet implemented; \
         see src/auth/session_cookie/verify.rs for details"
    )
}
