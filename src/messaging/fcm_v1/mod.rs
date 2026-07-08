//! Wire-format request/response DTOs and endpoint URL builders for the FCM
//! v1 REST API.
//!
//! Kept separate from [`crate::messaging::message`] so that Google's JSON
//! field names (`registration_token`, `collapse_key`, ...) never leak into
//! the crate's public API — mirrors `crate::auth::identity_toolkit`.

mod endpoints;
mod operations;
mod requests;

pub(crate) use endpoints::{FcmEndpoints, IidEndpoints};
pub(crate) use operations::MessagingOperations;
pub(crate) use requests::{IidResponse, SendRequest, SendResponse, WireMessage};
