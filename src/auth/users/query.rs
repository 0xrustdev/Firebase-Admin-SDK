//! Pagination helpers for `accounts:batchGet` (list users).

use crate::auth::users::model::UserRecord;

/// One page of a `list_users` call.
#[derive(Debug)]
pub struct UserPage {
    /// Users returned on this page.
    pub users: Vec<UserRecord>,
    /// Token to pass to the next `list_users` call, if more pages remain.
    pub next_page_token: Option<String>,
}
