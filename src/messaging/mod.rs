//! Firebase Cloud Messaging: sending messages and managing topic
//! subscriptions.

pub mod client;
pub mod error;
mod fcm_v1;
pub mod message;
#[cfg(feature = "live-messaging")]
mod token_provider;

pub use client::{MessagingClient, MessagingClientBuilder, MAX_BATCH_SIZE};
pub use error::MessagingError;
pub use message::{
    AndroidConfig, ApnsConfig, BatchResponse, Message, Notification, SendError, SendResult, Target,
    TopicManagementError, TopicManagementResponse, WebpushConfig, WebpushNotification,
};
