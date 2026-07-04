//! An open-source Firebase Admin SDK for Rust.
//!
//! `firebase-admin` currently implements Firebase **Authentication**:
//! verifying ID tokens, creating custom tokens, managing users, and
//! session cookies. Support for additional Firebase services (Firestore,
//! Cloud Storage, ...) is planned; see `ARCHITECTURE.md` in the repository
//! root for the project's module and versioning conventions.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() -> Result<(), firebase_admin::Error> {
//! use firebase_admin::auth::AuthClient;
//!
//! let auth = AuthClient::builder("my-project-id")
//!     .service_account_key(firebase_admin::core::ServiceAccountKey::from_file(
//!         "service-account.json",
//!     )?)
//!     .build()?;
//!
//! let claims = auth.verify_id_token("<id-token-from-client>").await?;
//! println!("verified uid: {}", claims.sub);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod auth;
pub mod core;
mod error;

pub use error::Error;

/// A `Result` alias using [`Error`] as its error type.
pub type Result<T> = std::result::Result<T, Error>;
