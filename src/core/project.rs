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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_non_empty_id() {
        let id = ProjectId::new("my-project").unwrap();
        assert_eq!(id.as_str(), "my-project");
        assert_eq!(id.to_string(), "my-project");
    }

    #[test]
    fn rejects_an_empty_id() {
        assert!(matches!(
            ProjectId::new(""),
            Err(CoreError::InvalidProjectId(_))
        ));
    }

    #[test]
    fn rejects_a_whitespace_only_id() {
        assert!(matches!(
            ProjectId::new("   "),
            Err(CoreError::InvalidProjectId(_))
        ));
    }
}
