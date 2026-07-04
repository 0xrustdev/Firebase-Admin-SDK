//! Integration tests against a running Firebase Auth Emulator.
//!
//! Skipped locally if `FIREBASE_AUTH_EMULATOR_HOST` is not set. Always run
//! in CI (see `.github/workflows/ci.yml`, job `emulator-integration`).

use firebase_admin::auth::{AuthClient, CreateUserRequest};

fn emulator_available() -> bool {
    std::env::var(firebase_admin::auth::mode::EMULATOR_HOST_ENV_VAR)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

#[tokio::test]
async fn emulator_create_get_delete_user_roundtrip() {
    if !emulator_available() {
        eprintln!("skipping: set FIREBASE_AUTH_EMULATOR_HOST to run emulator integration tests");
        return;
    }

    let auth = AuthClient::builder("demo-test-project")
        .build()
        .expect("failed to build AuthClient");

    let created = auth
        .create_user(CreateUserRequest {
            email: Some(format!("test-{}@example.com", uuid_like_suffix())),
            password: Some("correct horse battery staple".to_string()),
            ..Default::default()
        })
        .await
        .expect("create_user failed");

    let fetched = auth.get_user(&created.uid).await.expect("get_user failed");
    assert_eq!(fetched.uid, created.uid);

    auth.delete_user(&created.uid)
        .await
        .expect("delete_user failed");
}

fn uuid_like_suffix() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}
