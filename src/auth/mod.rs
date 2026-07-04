//! Firebase Authentication: ID token verification, custom token creation,
//! session cookies, and user management.

pub mod client;
pub mod custom_token;
pub mod error;
pub mod id_token;
pub mod identity_toolkit;
pub mod mode;
pub mod session_cookie;
pub mod users;

pub use client::{AuthClient, AuthClientBuilder};
pub use error::AuthError;
pub use mode::ClientMode;
pub use users::{CreateUserRequest, UpdateUserRequest, UserRecord};
