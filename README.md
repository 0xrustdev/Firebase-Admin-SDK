# firebase-admin-rs

[![CI](https://github.com/0xrustdev/Firebase-Admin-SDK/actions/workflows/ci.yml/badge.svg)](https://github.com/0xrustdev/Firebase-Admin-SDK/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/firebase-admin.svg)](https://crates.io/crates/firebase-admin)
[![docs.rs](https://docs.rs/firebase-admin/badge.svg)](https://docs.rs/firebase-admin)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

An open-source, community-maintained **Firebase Admin SDK for Rust**.

The Rust ecosystem has no mature, actively-maintained Firebase Admin SDK.
This project aims to fill that gap: starting with **Authentication** and
**Cloud Messaging**, and expanding feature-by-feature toward a full SDK
(Firestore, Cloud Storage, Realtime Database, Remote Config, ...).

## Status

**Pre-1.0, under active development.** The public API may change between
`0.x` releases. See the [roadmap](#roadmap) below.

## Features (Authentication)

- ID token verification (RS256, JWKS caching, full claim validation)
- Custom token creation (signed locally with a service account key)
- Session cookie creation and verification — verified against Google's
  session-cookie certificate endpoint, which is distinct from the ID-token
  JWKS endpoint (confirmed against the official Admin SDKs; see
  [`ARCHITECTURE.md`](ARCHITECTURE.md))
- User management (create, get, update, delete, list, custom claims) via the
  Identity Toolkit REST API, against both the **Firebase Auth Emulator** and
  **production Firebase** — OAuth2 bearer tokens are acquired automatically
  via [`gcp_auth`](https://crates.io/crates/gcp_auth) for both explicit
  service account keys and Application Default Credentials
- A single, unified client for both production Firebase and the local
  [Firebase Auth Emulator](https://firebase.google.com/docs/emulator-suite) —
  no divergent APIs or generic type parameters to juggle

All of the above work end-to-end against production Firebase when the
default `live-user-management` feature is enabled (it is on by default).

## Features (Cloud Messaging)

- Sending a single message to a device token, topic, or condition
  (`send`), with optional dry-run validation
- Sending up to 500 messages concurrently, each with its own
  success/failure result (`send_each`, `send_each_for_multicast`)
- Topic subscription management (`subscribe_to_topic`,
  `unsubscribe_from_topic`), reporting per-token failures rather than
  failing the whole call when only some tokens are rejected
- Full message configuration: notifications, custom data, and
  platform-specific delivery options for Android, APNs, and Web Push

Cloud Messaging has no emulator — every operation requires a live service
account or Application Default Credentials (the default `live-messaging`
feature).

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

```rust,no_run
use firebase_admin::messaging::{Message, MessagingClient, Notification};

#[tokio::main]
async fn main() -> Result<(), firebase_admin::Error> {
    let messaging = MessagingClient::builder("my-project-id")
        .application_default_credentials()
        .build()?;

    let message = Message::to_token("<device-registration-token>").with_notification(
        Notification {
            title: Some("Hello".to_string()),
            body: Some("This is a test notification".to_string()),
            image: None,
        },
    );

    let message_id = messaging.send(&message, false).await?;
    println!("sent message: {message_id}");

    Ok(())
}
```

More examples live in [`examples/`](examples/).

## Why another Firebase crate?

A few Firebase-related crates exist for Rust, but none combine ID token
verification, custom token creation, and full user management under one
well-documented, actively-maintained, community-oriented roof. This project
is built to be that crate — see [`ARCHITECTURE.md`](ARCHITECTURE.md) for the
design rationale, including why the live/emulator client is a single
concrete type rather than generic over credentials.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for a full, method-by-method checklist of this
crate's coverage against the official Firebase Admin SDK — what's done, what's
left, and an effort estimate for each unfinished item, across Authentication
and every other Firebase service (Firestore, Cloud Storage, Realtime
Database, Cloud Messaging, Remote Config, and more). See
[CHANGELOG.md](CHANGELOG.md) for what has actually shipped in each release.

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
