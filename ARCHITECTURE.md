# Architecture

## Why a single crate today

`firebase-admin` is a single crate, not a Cargo workspace, even though its
`src/` layout looks like a workspace-in-waiting. AWS's and Google's official
Rust SDKs split into one crate per service because they codegen hundreds of
services from machine-readable models and need independent compilation/
versioning at that scale. Neither applies here yet: v1 has two services
(Auth, Messaging). A premature workspace split would mean multiple
`Cargo.toml`s and cross-crate re-exports for little present benefit.

## The `core/` seam

`src/core/` holds everything that is *not* specific to a single Firebase
service: credential loading, the shared HTTP client wrapper, project ID
handling, and the base error type. It now has two consumers (`src/auth/` and
`src/messaging/`); when a third service (most likely Firestore) is added,
`src/core/` becomes the `firebase-admin-core` crate, `src/auth/` and
`src/messaging/` become their own crates, and a thin facade crate re-exports
all of them behind feature flags. Because the module boundary already
exists, that split is a mechanical `git mv` + path fix, not a redesign.

`Credentials::ApplicationDefault` (`src/core/credentials.rs`) is gated behind
`any(feature = "live-user-management", feature = "live-messaging")` rather
than a single flag, since it's now shared by both services' live-credential
paths but each service can be compiled out independently.

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

## Module map (`src/messaging/`)

- `client.rs` — `MessagingClient` and `MessagingClientBuilder`, the public
  entry point. Unlike `AuthClient`, there is no emulator mode: every FCM v1
  and Instance ID call requires a live OAuth2 bearer token, so the
  live/emulator `mode` field Auth needs doesn't exist here.
- `message.rs` — the ergonomic, Rust-facing message types (`Message`,
  `Notification`, `AndroidConfig`, `ApnsConfig`, `WebpushConfig`,
  `BatchResponse`, `TopicManagementResponse`, ...).
- `fcm_v1/` — wire-format DTOs and endpoint URL builders for the FCM v1
  (`fcm.googleapis.com`) and legacy Instance ID (`iid.googleapis.com`) REST
  APIs. Kept separate from `message.rs` for the same reason
  `identity_toolkit/` is kept separate from `users/`: Google's wire-format
  field names must never leak into the public API.
- `token_provider.rs` — OAuth2 bearer token acquisition, structurally
  identical to `auth/token_provider.rs` (see the `gcp_auth`
  service-account-JSON gotcha below). Feature-gated behind
  `live-messaging` (on by default).

### `send_each`/`send_each_for_multicast`: concurrent, not batched

FCM v1 has no true batch-send endpoint. `send_each` dispatches one HTTP
request per message and awaits all of them concurrently via
`futures_util::future::join_all`, mirroring the official Admin SDKs'
`Promise.allSettled` — confirmed against `firebase-admin-node`'s
`messaging.ts`. An earlier version of this code awaited each request
sequentially in a loop, which is correct but serializes every round-trip;
for a 500-message batch that's a meaningful latency regression relative to
the official SDK, so it was changed to dispatch-then-await-all. Batch size
over `MAX_BATCH_SIZE` (500) is rejected as a recoverable
`MessagingError::BatchTooLarge`, not an `assert!` — a caller passing bad
input should get a typed error, not a panicked task.

### Topic management: per-token results, not all-or-nothing

`subscribe_to_topic`/`unsubscribe_from_topic` call the Instance ID API's
`batchAdd`/`batchRemove`, which can partially fail — some tokens invalid,
others fine. The response is mapped into a `TopicManagementResponse` with
per-token `errors` (index + reason), not a single `Result<(), _>` that fails
the whole call on the first bad token; that would silently discard which
tokens actually succeeded. This mirrors the official SDKs'
`mapRawResponseToTopicManagementResponse`.

### The `access_token_auth` header

Every FCM v1 and Instance ID request carries an `access_token_auth: true`
header in addition to the OAuth2 `Authorization: Bearer` header
(`MessagingOperations::post` in `fcm_v1/operations.rs`). This isn't
documented in the FCM v1 REST reference; it was found by reading
`firebase-admin-node`'s `FirebaseMessagingRequestHandler`, which sends it
unconditionally on every messaging request. Some deployments of the legacy
Instance ID API reject otherwise-valid Bearer-authenticated requests
without it — a gap that would only surface as sporadic authorization
failures against production traffic, so it's sent unconditionally rather
than only when a failure is observed.

## The `gcp_auth` service-account JSON reconstruction gotcha

Both `auth/token_provider.rs` and `messaging/token_provider.rs` need to hand
a service-account key to `gcp_auth::CustomServiceAccount::from_json`, but
`crate::core::ServiceAccountKey` only stores the four fields this crate's
own code actually uses (`client_email`, `private_key`, `private_key_id`,
`project_id`) — it doesn't keep the original key file's JSON around. Both
modules reconstruct a minimal JSON object to hand to `gcp_auth`.

`gcp_auth` 0.12's internal deserialization target
(`gcp_auth::types::ServiceAccountKey`) requires `token_uri` as a
**non-optional** `String` field. A reconstructed JSON that omits it fails
with `missing field `token_uri`` — not from a malformed key, but because the
reconstruction never included it. This was caught by an external smoke test
using a real-shaped service account key rather than the crate's own
wiremock-backed unit tests, which never exercise the real `gcp_auth` parsing
path. `token_uri` is `https://oauth2.googleapis.com/token` for every
standard Google service account key (a fixed constant, not per-key data),
so both `token_provider.rs` files hardcode it into their reconstructed JSON
as `GOOGLE_OAUTH2_TOKEN_URI`. Each has a regression test
(`from_service_account_produces_json_gcp_auth_can_parse`) asserting the
reconstructed JSON round-trips through `gcp_auth` successfully — this is the
one behavior the crate's HTTP-mocked test suite structurally cannot catch,
since `gcp_auth` parsing happens entirely before any network call.

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
transport/credential/parsing failures, `AuthError`/`MessagingError` (each
wrapping `CoreError`) for everything specific to their service, and a
crate-root `Error` unifying every module's error type via `#[from]`. Adding
a new service later means adding a `FirestoreError` and one new
`Error::Firestore` variant — existing variants are never restructured.

## Versioning

See the roadmap in `README.md`. In short: `0.x` releases build out full Auth
and Messaging coverage; `1.0.0` means both are feature-complete and their
public APIs are stable; new Firebase services after that ship as additive
modules, triggering the workspace split described above.
