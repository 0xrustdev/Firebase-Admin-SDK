//! Wire-format request/response DTOs for the FCM v1 and Instance ID REST
//! APIs.
//!
//! These types mirror Google's JSON shapes exactly and are intentionally
//! kept separate from the ergonomic types in [`crate::messaging::message`]
//! so that REST field-naming quirks (`registration_token`, `collapse_key`,
//! ...) never leak into the crate's public API.

use crate::messaging::message::{
    AndroidConfig, ApnsConfig, Message, Notification, Target, WebpushConfig, WebpushNotification,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request body for `projects/{projectId}/messages:send`.
#[derive(Debug, Serialize)]
pub(crate) struct SendRequest {
    pub(crate) message: WireMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) validate_only: Option<bool>,
}

/// The `message` object within a [`SendRequest`].
#[derive(Debug, Default, Serialize)]
pub(crate) struct WireMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) notification: Option<WireNotification>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) data: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) android: Option<WireAndroidConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) apns: Option<WireApnsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) webpush: Option<WireWebpushConfig>,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct WireNotification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) image: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct WireAndroidConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ttl: Option<String>,
    #[serde(rename = "collapse_key", skip_serializing_if = "Option::is_none")]
    pub(crate) collapse_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) priority: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct WireApnsConfig {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) payload: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct WireWebpushConfig {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) data: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) notification: Option<serde_json::Map<String, serde_json::Value>>,
}

impl From<&Message> for WireMessage {
    fn from(message: &Message) -> Self {
        let (token, topic, condition) = match &message.target {
            Target::Token(t) => (Some(t.clone()), None, None),
            Target::Topic(t) => (None, Some(t.clone()), None),
            Target::Condition(c) => (None, None, Some(c.clone())),
        };

        Self {
            token,
            topic,
            condition,
            notification: message.notification.as_ref().map(WireNotification::from),
            data: message.data.clone(),
            android: message.android.as_ref().map(WireAndroidConfig::from),
            apns: message.apns.as_ref().map(WireApnsConfig::from),
            webpush: message.webpush.as_ref().map(WireWebpushConfig::from),
        }
    }
}

impl From<&Notification> for WireNotification {
    fn from(n: &Notification) -> Self {
        Self {
            title: n.title.clone(),
            body: n.body.clone(),
            image: n.image.clone(),
        }
    }
}

impl From<&AndroidConfig> for WireAndroidConfig {
    fn from(c: &AndroidConfig) -> Self {
        Self {
            ttl: c.ttl_seconds.map(|s| format!("{s}s")),
            collapse_key: c.collapse_key.clone(),
            priority: c.priority.clone(),
        }
    }
}

impl From<&ApnsConfig> for WireApnsConfig {
    fn from(c: &ApnsConfig) -> Self {
        Self {
            headers: c.headers.clone(),
            payload: c.payload.clone(),
        }
    }
}

impl From<&WebpushConfig> for WireWebpushConfig {
    fn from(c: &WebpushConfig) -> Self {
        Self {
            headers: c.headers.clone(),
            data: c.data.clone(),
            notification: c.notification.as_ref().map(webpush_notification_to_json),
        }
    }
}

/// Merges [`WebpushNotification`]'s named fields and `extra` map into a
/// single JSON object, matching the flat `webpush.notification` shape the
/// Web Notifications API (and therefore FCM's wire format) expects.
fn webpush_notification_to_json(
    n: &WebpushNotification,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = n.extra.clone();
    if let Some(title) = &n.title {
        map.insert(
            "title".to_string(),
            serde_json::Value::String(title.clone()),
        );
    }
    if let Some(body) = &n.body {
        map.insert("body".to_string(), serde_json::Value::String(body.clone()));
    }
    if let Some(icon) = &n.icon {
        map.insert("icon".to_string(), serde_json::Value::String(icon.clone()));
    }
    map
}

/// Response body for `projects/{projectId}/messages:send`.
#[derive(Debug, Deserialize)]
pub(crate) struct SendResponse {
    pub(crate) name: String,
}

/// Response body for `iid/v1:batchAdd` and `iid/v1:batchRemove`.
///
/// On full success, FCM returns `{"results": [{}]}` — one empty object per
/// input token. On partial failure, individual entries carry an `error`
/// field instead.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct IidResponse {
    #[serde(default)]
    pub(crate) results: Vec<IidErrorResult>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IidErrorResult {
    pub(crate) error: Option<String>,
}
