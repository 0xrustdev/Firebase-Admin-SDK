//! Demonstrates basic user management.
//!
//! Against the Firebase Auth Emulator (start it first with
//! `firebase emulators:start --only auth`):
//!
//! ```text
//! FIREBASE_AUTH_EMULATOR_HOST=localhost:9099 cargo run --example manage_users -- demo-project
//! ```
//!
//! Against production Firebase, set `GOOGLE_APPLICATION_CREDENTIALS` to a
//! service account key file and drop the `FIREBASE_AUTH_EMULATOR_HOST`
//! env var — no code changes needed, `AuthClient` detects the environment
//! the same way either way. See `examples/verify_id_token.rs` for
//! constructing a client from an explicit service account key file instead
//! of Application Default Credentials.

use firebase_admin::auth::{AuthClient, CreateUserRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "demo-project".to_string());

    let auth = AuthClient::builder(project_id).build()?;

    let user = auth
        .create_user(CreateUserRequest {
            email: Some("example@example.com".to_string()),
            password: Some("correct horse battery staple".to_string()),
            ..Default::default()
        })
        .await?;
    println!("created user: {} ({:?})", user.uid, user.email);

    let fetched = auth.get_user(&user.uid).await?;
    println!("fetched user: {} ({:?})", fetched.uid, fetched.email);

    auth.delete_user(&user.uid).await?;
    println!("deleted user: {}", user.uid);

    Ok(())
}
