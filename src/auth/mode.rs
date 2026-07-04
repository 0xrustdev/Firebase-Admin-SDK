//! Runtime live/emulator mode selection.
//!
//! Unlike an approach that makes the client generic over a credentials or
//! mode type (which forces every method signature to diverge between live
//! and emulator variants), [`ClientMode`] is a plain runtime value. Every
//! [`crate::auth::AuthClient`] method is defined exactly once and branches
//! internally on `self.mode` only where behavior genuinely differs.

use crate::auth::identity_toolkit::IdentityToolkitEndpoints;

/// The environment variable Firebase's own SDKs use to auto-detect a running
/// Auth Emulator.
pub const EMULATOR_HOST_ENV_VAR: &str = "FIREBASE_AUTH_EMULATOR_HOST";

/// Selects whether an [`crate::auth::AuthClient`] talks to production
/// Firebase or a local emulator instance.
#[derive(Debug, Clone)]
pub enum ClientMode {
    /// Talk to production Firebase Authentication.
    Live,
    /// Talk to a local Firebase Auth Emulator at `host` (e.g. `localhost:9099`).
    Emulator {
        /// The emulator's host and port.
        host: String,
    },
}

impl ClientMode {
    /// Resolves the mode to use: an explicitly-requested emulator host takes
    /// priority, then the `FIREBASE_AUTH_EMULATOR_HOST` environment variable,
    /// and otherwise [`ClientMode::Live`].
    pub fn resolve(explicit_emulator_host: Option<String>) -> Self {
        if let Some(host) = explicit_emulator_host {
            return ClientMode::Emulator { host };
        }
        if let Ok(host) = std::env::var(EMULATOR_HOST_ENV_VAR) {
            if !host.trim().is_empty() {
                return ClientMode::Emulator { host };
            }
        }
        ClientMode::Live
    }

    /// Builds the Identity Toolkit endpoint set for this mode.
    pub fn endpoints(&self) -> IdentityToolkitEndpoints {
        match self {
            ClientMode::Live => IdentityToolkitEndpoints::live(),
            ClientMode::Emulator { host } => IdentityToolkitEndpoints::emulator(host),
        }
    }

    /// Whether requests in this mode require an OAuth2 bearer token.
    ///
    /// The Firebase Auth Emulator does not enforce authentication.
    pub fn requires_bearer_token(&self) -> bool {
        matches!(self, ClientMode::Live)
    }
}
