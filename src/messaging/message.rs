//! Ergonomic, Rust-facing message types.
//!
//! These are distinct from the crate-internal wire-format DTOs used to talk
//! to the FCM v1 REST API (see `crate::messaging::fcm_v1`), so that the
//! public API never exposes Google's REST field naming or shapes directly.

use std::collections::HashMap;

/// A message to send via Firebase Cloud Messaging.
///
/// Exactly one of [`Self::target`] must be set to a token, topic, or
/// condition — this is enforced by [`Target`] being a single field rather
/// than three optional ones.
#[derive(Debug, Clone)]
pub struct Message {
    /// Who the message is delivered to.
    pub target: Target,
    /// A simple, display notification shown by the client's OS.
    pub notification: Option<Notification>,
    /// Custom key-value payload delivered to the client app.
    pub data: HashMap<String, String>,
    /// Android-specific delivery options.
    pub android: Option<AndroidConfig>,
    /// Apple Push Notification Service-specific delivery options, as a raw
    /// APNs payload (`aps` dictionary and custom keys).
    pub apns: Option<ApnsConfig>,
    /// Web Push-specific delivery options.
    pub webpush: Option<WebpushConfig>,
}

impl Message {
    /// Starts building a message addressed to a single device registration
    /// token.
    pub fn to_token(token: impl Into<String>) -> Self {
        Self::new(Target::Token(token.into()))
    }

    /// Starts building a message addressed to a topic.
    pub fn to_topic(topic: impl Into<String>) -> Self {
        Self::new(Target::Topic(topic.into()))
    }

    /// Starts building a message addressed to devices matching a topic
    /// condition expression, e.g. `"'A' in topics && 'B' in topics"`.
    pub fn to_condition(condition: impl Into<String>) -> Self {
        Self::new(Target::Condition(condition.into()))
    }

    fn new(target: Target) -> Self {
        Self {
            target,
            notification: None,
            data: HashMap::new(),
            android: None,
            apns: None,
            webpush: None,
        }
    }

    /// Sets a display notification.
    pub fn with_notification(mut self, notification: Notification) -> Self {
        self.notification = Some(notification);
        self
    }

    /// Sets the custom data payload, replacing any previous value.
    pub fn with_data(mut self, data: HashMap<String, String>) -> Self {
        self.data = data;
        self
    }

    /// Sets Android-specific delivery options.
    pub fn with_android(mut self, android: AndroidConfig) -> Self {
        self.android = Some(android);
        self
    }

    /// Sets APNs-specific delivery options.
    pub fn with_apns(mut self, apns: ApnsConfig) -> Self {
        self.apns = Some(apns);
        self
    }

    /// Sets Web Push-specific delivery options.
    pub fn with_webpush(mut self, webpush: WebpushConfig) -> Self {
        self.webpush = Some(webpush);
        self
    }
}

/// Who an FCM [`Message`] is delivered to.
#[derive(Debug, Clone)]
pub enum Target {
    /// A single device registration token.
    Token(String),
    /// All devices subscribed to a topic.
    Topic(String),
    /// All devices matching a topic condition expression.
    Condition(String),
}

/// A simple, display notification shown by the client's OS.
#[derive(Debug, Default, Clone)]
pub struct Notification {
    /// The notification's title.
    pub title: Option<String>,
    /// The notification's body text.
    pub body: Option<String>,
    /// A URL to an image to display in the notification.
    pub image: Option<String>,
}

/// Android-specific delivery options for a [`Message`].
#[derive(Debug, Default, Clone)]
pub struct AndroidConfig {
    /// How long (in seconds) the message should be kept on FCM storage while
    /// the device is offline.
    pub ttl_seconds: Option<u64>,
    /// An identifier used to collapse a group of messages, keeping only the
    /// last one.
    pub collapse_key: Option<String>,
    /// Delivery priority: `"normal"` or `"high"`.
    pub priority: Option<String>,
}

/// Apple Push Notification Service-specific delivery options for a
/// [`Message`], as a raw APNs payload.
#[derive(Debug, Default, Clone)]
pub struct ApnsConfig {
    /// Raw APNs headers, e.g. `apns-priority`, `apns-expiration`.
    pub headers: HashMap<String, String>,
    /// The raw APNs JSON payload, including the `aps` dictionary.
    pub payload: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Web Push-specific delivery options for a [`Message`].
#[derive(Debug, Default, Clone)]
pub struct WebpushConfig {
    /// Raw Web Push protocol headers, e.g. `Urgency`, `TTL`.
    pub headers: HashMap<String, String>,
    /// Custom key-value payload delivered to the Web Push client, merged
    /// with (and taking precedence over) [`Message::data`].
    pub data: HashMap<String, String>,
    /// A display notification shown by the browser, using the Web
    /// Notifications API shape rather than [`Notification`]'s simplified
    /// title/body/image.
    pub notification: Option<WebpushNotification>,
}

/// A Web Push display notification, following the Web Notifications API
/// (`https://developer.mozilla.org/docs/Web/API/Notification`) — the shape
/// FCM forwards verbatim as `webpush.notification` to the browser's push
/// event, distinct from [`Notification`]'s cross-platform title/body/image.
#[derive(Debug, Default, Clone)]
pub struct WebpushNotification {
    /// The notification's title.
    pub title: Option<String>,
    /// The notification's body text.
    pub body: Option<String>,
    /// A URL to an icon to display in the notification.
    pub icon: Option<String>,
    /// Any additional Web Notification fields not covered above (e.g.
    /// `badge`, `image`, `actions`, `vibrate`, `tag`), merged directly into
    /// the wire-format `webpush.notification` object.
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The outcome of sending a single [`Message`] as part of a batch
/// ([`crate::messaging::MessagingClient::send_each`] or
/// [`crate::messaging::MessagingClient::send_each_for_multicast`]).
#[derive(Debug, Clone)]
pub enum SendResult {
    /// The message was accepted; carries FCM's message id.
    Success {
        /// The FCM-assigned identifier for the sent message.
        message_id: String,
    },
    /// The message was rejected.
    Failure {
        /// The error returned for this particular message.
        error: SendError,
    },
}

/// The error for a single failed message within a batch send.
#[derive(Debug, Clone)]
pub struct SendError {
    /// HTTP status code returned for this message.
    pub status: u16,
    /// Human-readable error message.
    pub message: String,
    /// Machine-readable error code, when FCM provides one (e.g.
    /// `UNREGISTERED`, `INVALID_ARGUMENT`, `QUOTA_EXCEEDED`).
    pub error_code: Option<String>,
}

/// The response to [`crate::messaging::MessagingClient::send_each`] or
/// [`crate::messaging::MessagingClient::send_each_for_multicast`].
#[derive(Debug, Clone)]
pub struct BatchResponse {
    /// One result per input message, in the same order.
    pub responses: Vec<SendResult>,
    /// The number of messages that were successfully sent.
    pub success_count: usize,
    /// The number of messages that failed to send.
    pub failure_count: usize,
}

impl BatchResponse {
    pub(crate) fn from_results(responses: Vec<SendResult>) -> Self {
        let success_count = responses
            .iter()
            .filter(|r| matches!(r, SendResult::Success { .. }))
            .count();
        let failure_count = responses.len() - success_count;
        Self {
            responses,
            success_count,
            failure_count,
        }
    }
}

/// The response to
/// [`crate::messaging::MessagingClient::subscribe_to_topic`] or
/// [`crate::messaging::MessagingClient::unsubscribe_from_topic`].
///
/// A single call can partially fail: some registration tokens may be
/// invalid or unregistered while others succeed. Mirrors the official Admin
/// SDKs' `TopicManagementResponse`, which reports per-token errors rather
/// than failing the whole call when only some tokens are rejected.
#[derive(Debug, Clone)]
pub struct TopicManagementResponse {
    /// The number of registration tokens that were successfully
    /// subscribed/unsubscribed.
    pub success_count: usize,
    /// The number of registration tokens that failed.
    pub failure_count: usize,
    /// One entry per failed registration token, in the order they were
    /// passed in.
    pub errors: Vec<TopicManagementError>,
}

/// A single failed registration token within a [`TopicManagementResponse`].
#[derive(Debug, Clone)]
pub struct TopicManagementError {
    /// The position of the failed token within the input slice.
    pub index: usize,
    /// The short error reason FCM returned for this token (e.g.
    /// `NOT_FOUND`, `INVALID_ARGUMENT`).
    pub reason: String,
}

impl TopicManagementResponse {
    pub(crate) fn from_errors(total: usize, errors: Vec<TopicManagementError>) -> Self {
        Self {
            success_count: total - errors.len(),
            failure_count: errors.len(),
            errors,
        }
    }
}
