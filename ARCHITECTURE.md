# Architecture

## Why a single crate today

`firebase-admin` is a single crate, not a Cargo workspace, even though its
`src/` layout looks like a workspace-in-waiting. AWS's and Google's official
Rust SDKs split into one crate per service because they codegen hundreds of
services from machine-readable models and need independent compilation/
versioning at that scale. Neither applies here yet: v1 has exactly one
service (Auth). A premature workspace split would mean three `Cargo.toml`s
and cross-crate re-exports for zero present benefit.

## The `core/` seam

`src/core/` holds everything that is *not* specific to Authentication:
credential loading, the shared HTTP client wrapper, project ID handling, and
the base error type. It has only one consumer today (`src/auth/`), which can
look over-engineered for a single-service crate — it is intentional. When a
second service (most likely Firestore) is added, `src/core/` becomes the
`firebase-admin-core` crate, `src/auth/` becomes `firebase-admin-auth`, and a
thin facade crate re-exports both behind feature flags. Because the module
boundary already exists, that split is a mechanical `git mv` + path fix, not
a redesign.

## Module map (`src/auth/`)

- `client.rs` — `AuthClient` and `AuthClientBuilder`, the public entry point.
- `mode.rs` — `ClientMode`, the runtime enum that unifies live and emulator
  behavior (see below).
- `id_token/` — ID token claim shapes, JWKS fetching/caching, and the
  verifier itself.
- `custom_token/` — local RS256 signing of Firebase custom tokens.
- `session_cookie/` — session cookie creation and verification. Verification
  is a near-duplicate of ID token verification (same RS256/exp/aud checks,
  reused via `id_token::verifier::verify_with_key`'s parameterized issuer
  prefix) but against a *different* key set: `certs.rs` fetches X.509
  certificates from `identitytoolkit/v3/relyingparty/publicKeys`, not the
  JWK-format securetoken endpoint ID tokens use. This was confirmed by
  reading the official Node.js/Python Admin SDK source and by fetching both
  endpoints directly — the two token types are not verified against the same
  keys, and assuming otherwise would silently break session cookie
  verification the day Google rotates one key set but not the other.
- `token_provider.rs` — OAuth2 bearer token acquisition for live-mode
  Identity Toolkit calls, via `gcp_auth`. Feature-gated behind
  `live-user-management` (on by default); only compiled in when that feature
  is enabled, so consumers who only verify tokens don't pay for it.
- `users/` — the ergonomic, Rust-facing user management API
  (`UserRecord`, `CreateUserRequest`, ...).
- `identity_toolkit/` — wire-format DTOs and endpoint URL builders for
  Google's Identity Toolkit REST API. Kept separate from `users/` so Google's
  JSON field names (`localId`, `customAttributes`, ...) never leak into the
  public API.

## Live vs. emulator: a runtime enum, not a generic type

A known competing crate makes its client generic over a credentials type,
which forces every method to exist in two variants with diverging
signatures for live vs. emulator use. `firebase-admin` avoids this: `mode` is
a plain field of type `ClientMode` (`Live` or `Emulator { host }`) on the one
concrete `AuthClient` struct. Every public method is defined exactly once and
branches on `self.mode` only where behavior genuinely differs — base URL,
whether an OAuth2 bearer token is required, which credentials are valid. Most
operations (e.g. custom token signing) don't branch on mode at all.

## X.509 certificate parsing for session cookies

`jsonwebtoken::DecodingKey::from_rsa_pem` technically accepts a PEM block
labeled `CERTIFICATE`, but it does not parse the X.509 `Certificate`
structure — it walks the raw DER for the first RSA/EC/Ed25519 OID it finds
and treats what follows as key material. That happens to locate the right
bytes for typical certificates, but it isn't a structural guarantee, and this
is a signature-verification trust boundary. `session_cookie/certs.rs`
instead uses the `x509-parser` crate to properly parse each certificate and
extract `tbs_certificate.subject_pki`, and strips the DER `INTEGER` encoding's
optional leading zero byte before base64url-encoding — omitting that step
would silently corrupt any RSA modulus whose leading bit happens to be 1
(the common case). This is covered by a test that performs a full
sign-with-known-key → extract-via-`certs.rs` → verify-with-extracted-key
round trip, not just a "did it error" check.

## Error handling

Errors are nested `thiserror` enums, not one flat enum: `CoreError` for
transport/credential/parsing failures, `AuthError` (wrapping `CoreError`) for
everything Auth-specific, and a crate-root `Error` unifying every module's
error type via `#[from]`. Adding a new service later means adding a
`FirestoreError` and one new `Error::Firestore` variant — existing variants
are never restructured.

## Versioning

See the roadmap in `README.md`. In short: `0.x` releases build out full Auth
coverage; `1.0.0` means Auth is feature-complete and its public API is
stable; new Firebase services after that ship as additive modules,
triggering the workspace split described above.
