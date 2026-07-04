//! Minimal end-to-end smoke test against the Firebase Auth Emulator.
//!
//! ```text
//! firebase emulators:start --only auth --project demo-project &
//! FIREBASE_AUTH_EMULATOR_HOST=localhost:9099 cargo run --example emulator_quickstart
//! ```

use firebase_admin::auth::AuthClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = AuthClient::builder("demo-project").build()?;

    let page = auth.list_users(10, None).await?;
    println!("found {} existing user(s)", page.users.len());

    Ok(())
}
