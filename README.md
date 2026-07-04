# firebase-admin-rs

[![CI](https://github.com/USERNAME/firebase-admin-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/USERNAME/firebase-admin-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/firebase-admin.svg)](https://crates.io/crates/firebase-admin)
[![docs.rs](https://docs.rs/firebase-admin/badge.svg)](https://docs.rs/firebase-admin)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

An open-source, community-maintained **Firebase Admin SDK for Rust**.

The Rust ecosystem has no mature, actively-maintained Firebase Admin SDK.
This project aims to fill that gap: starting with **Authentication**, and
expanding feature-by-feature toward a full SDK (Firestore, Cloud Storage,
Realtime Database, Cloud Messaging, ...).

## Status

**Pre-1.0, under active development.** The public API may change between
`0.x` releases. See the [roadmap](#roadmap) below.

## Features (Authentication)

- ID token verification (RS256, JWKS caching, full claim validation)
- Custom token creation (signed locally with a service account key)
- User management (create, get, update, delete, list) via the Identity
  Toolkit REST API — against the **Firebase Auth Emulator**
- Custom claims
- Session cookie creation (verification: see roadmap)
- A single, unified client for both production Firebase and the local
  [Firebase Auth Emulator](https://firebase.google.com/docs/emulator-suite) —
  no divergent APIs or generic type parameters to juggle

### Not yet implemented

- **User management against production Firebase.** Calls like `get_user`/
  `create_user`/etc. require an OAuth2 bearer token when talking to live
  Firebase; that token-exchange step isn't implemented yet, so these calls
  currently return a clear error in live mode. They work today against the
  Firebase Auth Emulator, which doesn't require authentication. Tracked for
  `v0.2.0`.
- **Application Default Credentials.** The `application-default-credentials`
  feature flag and `AuthClientBuilder::application_default_credentials()`
  exist but are not functional yet (same underlying gap as above); the
  feature is off by default until this lands.
- **Session cookie verification** (`verify_session_cookie`) — creation works;
  verification is an explicit `unimplemented!()` stub pending confirmation of
  which certificate endpoint Firebase uses for session cookies. Tracked for
  `v0.2.0`.

## Quick start

```rust,no_run
use firebase_admin::auth::AuthClient;
use firebase_admin::core::ServiceAccountKey;

#[tokio::main]
async fn main() -> Result<(), firebase_admin::Error> {
    let key = ServiceAccountKey::from_file("service-account.json")?;
    let auth = AuthClient::builder("my-project-id")
        .service_account_key(key)
        .build()?;

    let claims = auth.verify_id_token("<id-token-from-client>").await?;
    println!("verified uid: {}", claims.sub);

    Ok(())
}
```

Switching to the local emulator requires no code changes — just set
`FIREBASE_AUTH_EMULATOR_HOST=localhost:9099` in your environment, or call
`.use_emulator("localhost:9099")` on the builder explicitly.

More examples live in [`examples/`](examples/).

## Why another Firebase crate?

A few Firebase-related crates exist for Rust, but none combine ID token
verification, custom token creation, and full user management under one
well-documented, actively-maintained, community-oriented roof. This project
is built to be that crate — see [`ARCHITECTURE.md`](ARCHITECTURE.md) for the
design rationale, including why the live/emulator client is a single
concrete type rather than generic over credentials.

## Roadmap

- **v0.1.0** — ID token verification, custom token creation, basic user CRUD.
- **v0.2.0** — Session cookie verification, first-class custom-claims API,
  full emulator parity.
- **v0.3.0** — Bulk user import, email action links, initial multi-tenancy.
- **v1.0.0** — Feature-complete Auth, stable public API, full documentation
  coverage, security review of the token-verification path.
- **Post-1.0** — Firestore, then Cloud Storage, Realtime Database, Cloud
  Messaging, Remote Config, as additive modules.

## Contributing

Contributions are very welcome — see [`CONTRIBUTING.md`](CONTRIBUTING.md) to
get started, and [`ARCHITECTURE.md`](ARCHITECTURE.md) to understand the
module layout. Please also read the [Code of Conduct](CODE_OF_CONDUCT.md).

## Security

See [`SECURITY.md`](SECURITY.md) for how to report vulnerabilities.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
