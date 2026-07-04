//! The crate-root error type, unifying every service module's errors.

use crate::auth::AuthError;
use crate::core::CoreError;

/// The top-level error type for `firebase-admin`.
///
/// As additional Firebase services are added, each gets its own module error
/// type and a corresponding variant here — existing variants are never
/// restructured.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error from the `auth` module.
    #[error(transparent)]
    Auth(#[from] AuthError),

    /// A service-independent core error.
    #[error(transparent)]
    Core(#[from] CoreError),
}
