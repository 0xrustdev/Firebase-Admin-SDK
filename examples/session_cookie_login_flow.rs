//! Demonstrates a typical server-side session-cookie login flow: exchange a
//! client-supplied ID token for a long-lived session cookie, then later
//! verify that cookie on subsequent requests.
//!
//! ```text
//! cargo run --example session_cookie_login_flow -- <service-account.json> <project-id> <id-token>
//! ```

use firebase_admin::auth::AuthClient;
use firebase_admin::core::ServiceAccountKey;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let key_path = args
        .next()
        .expect("usage: <key.json> <project-id> <id-token>");
    let project_id = args
        .next()
        .expect("usage: <key.json> <project-id> <id-token>");
    let id_token = args
        .next()
        .expect("usage: <key.json> <project-id> <id-token>");

    let key = ServiceAccountKey::from_file(key_path)?;
    let auth = AuthClient::builder(project_id)
        .service_account_key(key)
        .build()?;

    // A client (mobile app, web SPA) signs in and sends its ID token here.
    // Exchange it for a session cookie valid for 5 days, and set that as an
    // HttpOnly cookie in the real response.
    let session_cookie = auth
        .create_session_cookie(&id_token, Duration::from_secs(5 * 24 * 60 * 60))
        .await?;
    println!("session cookie: {session_cookie}");

    // On a later request, the browser sends the cookie back; verify it to
    // authenticate the request. This is checked against a different
    // certificate endpoint and issuer than an ID token — see
    // `ARCHITECTURE.md` for why the two aren't interchangeable.
    let claims = auth.verify_session_cookie(&session_cookie).await?;
    println!("verified session for uid: {}", claims.sub);

    Ok(())
}
