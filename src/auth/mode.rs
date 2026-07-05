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

    /// The dummy `key=` query parameter value to attach to Identity Toolkit
    /// REST calls in emulator mode, or `None` in live mode.
    ///
    /// Every Identity Toolkit v1 `accounts:*` endpoint requires an API key
    /// query parameter to be *present*, even on the emulator — it isn't
    /// validated there, but its absence produces the same
    /// `PERMISSION_DENIED` / "The request is missing a valid API key." error
    /// the production API returns for unauthenticated calls. Production
    /// calls instead rely solely on the OAuth2 bearer token (see
    /// [`Self::requires_bearer_token`]) and must not send this parameter.
    pub fn emulator_api_key(&self) -> Option<&'static str> {
        match self {
            ClientMode::Live => None,
            ClientMode::Emulator { .. } => Some("fake-api-key"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `std::env::var` reads process-global state; serialize tests that touch
    // `EMULATOR_HOST_ENV_VAR` so they can't observe each other's values when
    // the test binary runs them concurrently.
    static ENV_VAR_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_emulator_env_var<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_VAR_TEST_LOCK.lock().unwrap();
        let previous = std::env::var(EMULATOR_HOST_ENV_VAR).ok();
        match value {
            Some(v) => std::env::set_var(EMULATOR_HOST_ENV_VAR, v),
            None => std::env::remove_var(EMULATOR_HOST_ENV_VAR),
        }

        let result = f();

        match previous {
            Some(v) => std::env::set_var(EMULATOR_HOST_ENV_VAR, v),
            None => std::env::remove_var(EMULATOR_HOST_ENV_VAR),
        }
        result
    }

    #[test]
    fn explicit_host_wins_over_env_var() {
        with_emulator_env_var(Some("env-host:9099"), || {
            let mode = ClientMode::resolve(Some("explicit-host:9099".to_string()));
            assert!(matches!(mode, ClientMode::Emulator { host } if host == "explicit-host:9099"));
        });
    }

    #[test]
    fn env_var_is_used_when_no_explicit_host_given() {
        with_emulator_env_var(Some("env-host:9099"), || {
            let mode = ClientMode::resolve(None);
            assert!(matches!(mode, ClientMode::Emulator { host } if host == "env-host:9099"));
        });
    }

    #[test]
    fn whitespace_only_env_var_is_treated_as_unset() {
        with_emulator_env_var(Some("   "), || {
            let mode = ClientMode::resolve(None);
            assert!(matches!(mode, ClientMode::Live));
        });
    }

    #[test]
    fn defaults_to_live_with_nothing_set() {
        with_emulator_env_var(None, || {
            let mode = ClientMode::resolve(None);
            assert!(matches!(mode, ClientMode::Live));
        });
    }

    #[test]
    fn requires_bearer_token_only_in_live_mode() {
        assert!(ClientMode::Live.requires_bearer_token());
        assert!(!ClientMode::Emulator {
            host: "localhost:9099".to_string()
        }
        .requires_bearer_token());
    }
}
