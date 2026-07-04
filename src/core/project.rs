//! Firebase/GCP project identifier handling.

use crate::core::error::CoreError;

/// A validated Firebase/GCP project identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    /// Creates a new [`ProjectId`], rejecting empty strings.
    pub fn new(id: impl Into<String>) -> Result<Self, CoreError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(CoreError::InvalidProjectId(
                "project id must not be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    /// Returns the project id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
