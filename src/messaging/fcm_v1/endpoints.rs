//! URL builders for the FCM v1 REST API.
//!
//! # Implementation status
//!
//! Paths below match the FCM v1 REST reference as of this writing, but must
//! be re-confirmed against
//! <https://firebase.google.com/docs/reference/fcm/rest/v1/projects.messages>
//! before relying on them — do not assume they are exact.

const FCM_BASE: &str = "https://fcm.googleapis.com/v1";
const IID_BASE: &str = "https://iid.googleapis.com";

/// Builds FCM v1 REST endpoint URLs for a given project.
pub(crate) struct FcmEndpoints {
    base: String,
    project_id: String,
}

impl FcmEndpoints {
    /// Endpoints pointed at the production FCM v1 API.
    pub(crate) fn live(project_id: &str) -> Self {
        Self {
            base: FCM_BASE.to_string(),
            project_id: project_id.to_string(),
        }
    }

    /// Endpoints pointed at an arbitrary base URL.
    ///
    /// Used by tests to point at a mock HTTP server; not exposed outside the
    /// crate since real callers should use [`Self::live`].
    #[cfg(test)]
    pub(crate) fn custom(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            project_id: "test-project".to_string(),
        }
    }

    /// `projects/{projectId}/messages:send` — send a single message.
    pub(crate) fn send(&self) -> String {
        format!("{}/projects/{}/messages:send", self.base, self.project_id)
    }
}

/// Instance ID API endpoint for topic subscription management.
///
/// Not versioned under `fcm_v1`'s project-scoped base URL: topic
/// subscribe/unsubscribe is served by the older `iid.googleapis.com` API,
/// not `fcm.googleapis.com/v1` — confirmed against
/// <https://firebase.google.com/docs/cloud-messaging/manage-topics>, which
/// documents no v1 equivalent for these two operations.
pub(crate) struct IidEndpoints {
    base: String,
}

impl IidEndpoints {
    /// Endpoints pointed at the production Instance ID API.
    pub(crate) fn live() -> Self {
        Self {
            base: IID_BASE.to_string(),
        }
    }

    /// Endpoints pointed at an arbitrary base URL, for tests.
    #[cfg(test)]
    pub(crate) fn custom(base: impl Into<String>) -> Self {
        Self { base: base.into() }
    }

    /// Subscribes registration tokens to a topic.
    pub(crate) fn batch_add(&self) -> String {
        format!("{}/iid/v1:batchAdd", self.base)
    }

    /// Unsubscribes registration tokens from a topic.
    pub(crate) fn batch_remove(&self) -> String {
        format!("{}/iid/v1:batchRemove", self.base)
    }
}
