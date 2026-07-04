//! Ergonomic user management API, backed by the Identity Toolkit REST API.

pub mod model;
pub mod operations;
pub mod query;

pub use model::{CreateUserRequest, UpdateUserRequest, UserRecord};
pub use operations::UserOperations;
pub use query::UserPage;
