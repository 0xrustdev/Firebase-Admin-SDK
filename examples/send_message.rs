//! Demonstrates sending a single FCM message, batch sending, and topic
//! subscription management.
//!
//! Requires a real service account key (FCM has no emulator mode):
//!
//! ```text
//! GOOGLE_APPLICATION_CREDENTIALS=service-account.json cargo run --example send_message -- my-project-id device-token
//! ```

use firebase_admin::messaging::{Message, MessagingClient, Notification};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let project_id = args
        .next()
        .expect("usage: send_message <project-id> <device-token>");
    let device_token = args
        .next()
        .expect("usage: send_message <project-id> <device-token>");

    let messaging = MessagingClient::builder(project_id)
        .application_default_credentials()
        .build()?;

    let message = Message::to_token(&device_token).with_notification(Notification {
        title: Some("Hello".to_string()),
        body: Some("This is a test message from firebase-admin-rs".to_string()),
        image: None,
    });

    // dry_run = true validates the message without delivering it.
    let message_id = messaging.send(&message, true).await?;
    println!("dry-run send accepted, would-be message id: {message_id}");

    // send_each_for_multicast: same message content to several tokens,
    // each delivered as its own request (concurrently), with a
    // per-token success/failure result.
    let batch = messaging
        .send_each_for_multicast(&message, std::slice::from_ref(&device_token), true)
        .await?;
    println!(
        "send_each_for_multicast: {} succeeded, {} failed",
        batch.success_count, batch.failure_count
    );

    // Topic management calls can partially fail (e.g. some tokens invalid);
    // check `failure_count`/`errors` rather than only the outer `Result`.
    let subscribe_result = messaging
        .subscribe_to_topic(std::slice::from_ref(&device_token), "news")
        .await?;
    println!(
        "subscribed to topic 'news': {} succeeded, {} failed ({:?})",
        subscribe_result.success_count, subscribe_result.failure_count, subscribe_result.errors
    );

    let unsubscribe_result = messaging
        .unsubscribe_from_topic(&[device_token], "news")
        .await?;
    println!(
        "unsubscribed from topic 'news': {} succeeded, {} failed ({:?})",
        unsubscribe_result.success_count,
        unsubscribe_result.failure_count,
        unsubscribe_result.errors
    );

    Ok(())
}
