//! Creates a Firebase custom token for a given uid.
//!
//! ```text
//! cargo run --example create_custom_token -- <service-account.json> <project-id> <uid>
//! ```

use firebase_admin::auth::AuthClient;
use firebase_admin::core::ServiceAccountKey;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let key_path = args.next().expect("usage: <key.json> <project-id> <uid>");
    let project_id = args.next().expect("usage: <key.json> <project-id> <uid>");
    let uid = args.next().expect("usage: <key.json> <project-id> <uid>");

    let key = ServiceAccountKey::from_file(key_path)?;
    let auth = AuthClient::builder(project_id)
        .service_account_key(key)
        .build()?;

    let token = auth.create_custom_token(&uid, None)?;
    println!("{token}");

    Ok(())
}
