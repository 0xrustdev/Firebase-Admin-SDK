//! Cross-service functionality shared by every Firebase service module.
//!
//! This module is the seam along which the crate will eventually be split
//! into a Cargo workspace (`firebase-admin-core` + one crate per service)
//! once a second Firebase service is added. See `ARCHITECTURE.md`.

pub mod credentials;
pub mod error;
pub mod http;
pub mod project;

pub use credentials::{Credentials, ServiceAccountKey};
pub use error::CoreError;
pub use http::HttpClient;
pub use project::ProjectId;
