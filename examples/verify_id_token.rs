//! Verifies a Firebase ID token using a service account key.
//!
//! ```text
//! cargo run --example verify_id_token -- <service-account.json> <project-id> <id-token>
//! ```

use firebase_admin::auth::AuthClient;
use firebase_admin::core::ServiceAccountKey;

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

    let claims = auth.verify_id_token(&id_token).await?;
    println!("verified uid: {}", claims.sub);

    Ok(())
}
