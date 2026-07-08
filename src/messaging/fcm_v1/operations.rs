//! Messaging operations against the FCM v1 and Instance ID REST APIs.

use crate::core::HttpClient;
use crate::messaging::error::{parse_fcm_response, MessagingError};
use crate::messaging::fcm_v1::{
    FcmEndpoints, IidEndpoints, IidResponse, SendRequest, SendResponse, WireMessage,
};
use crate::messaging::message::{
    Message, SendError, SendResult, TopicManagementError, TopicManagementResponse,
};

/// Performs messaging calls against the FCM v1 and Instance ID REST APIs.
///
/// Requires an OAuth2 bearer token (obtained from the configured service
/// account or Application Default Credentials) on every request — unlike
/// Identity Toolkit, FCM has no unauthenticated emulator mode.
pub(crate) struct MessagingOperations<'a> {
    http: &'a HttpClient,
    fcm_endpoints: &'a FcmEndpoints,
    iid_endpoints: &'a IidEndpoints,
    bearer_token: &'a str,
}

impl<'a> MessagingOperations<'a> {
    pub(crate) fn new(
        http: &'a HttpClient,
        fcm_endpoints: &'a FcmEndpoints,
        iid_endpoints: &'a IidEndpoints,
        bearer_token: &'a str,
    ) -> Self {
        Self {
            http,
            fcm_endpoints,
            iid_endpoints,
            bearer_token,
        }
    }

    /// Builds a `POST` request carrying the bearer token plus the
    /// `access_token_auth: true` header the official Admin SDKs send on
    /// every FCM v1 and Instance ID call
    /// (`FirebaseMessagingRequestHandler` in `firebase-admin-node`) — some
    /// deployments of the legacy Instance ID API reject otherwise-valid
    /// Bearer-authenticated requests without it.
    fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.http
            .inner()
            .post(url)
            .bearer_auth(self.bearer_token)
            .header("access_token_auth", "true")
    }

    /// Sends a single message, returning FCM's assigned message id.
    pub(crate) async fn send(
        &self,
        message: &Message,
        dry_run: bool,
    ) -> Result<String, MessagingError> {
        let body = SendRequest {
            message: WireMessage::from(message),
            validate_only: dry_run.then_some(true),
        };
        let response = self
            .post(&self.fcm_endpoints.send())
            .json(&body)
            .send()
            .await?;
        let parsed: SendResponse = parse_fcm_response(response).await?;
        Ok(parsed.name)
    }

    /// Sends a single message as part of a batch, converting any per-message
    /// failure into a [`SendResult::Failure`] rather than propagating it, so
    /// one bad message doesn't abort the rest of the batch.
    pub(crate) async fn send_for_batch(&self, message: &Message, dry_run: bool) -> SendResult {
        match self.send(message, dry_run).await {
            Ok(message_id) => SendResult::Success { message_id },
            Err(MessagingError::Api {
                status,
                message,
                error_code,
            }) => SendResult::Failure {
                error: SendError {
                    status,
                    message,
                    error_code,
                },
            },
            Err(other) => SendResult::Failure {
                error: SendError {
                    status: 0,
                    message: other.to_string(),
                    error_code: None,
                },
            },
        }
    }

    /// Subscribes registration tokens to a topic.
    pub(crate) async fn subscribe_to_topic(
        &self,
        tokens: &[String],
        topic: &str,
    ) -> Result<TopicManagementResponse, MessagingError> {
        self.batch_topic_operation(self.iid_endpoints.batch_add(), tokens, topic)
            .await
    }

    /// Unsubscribes registration tokens from a topic.
    pub(crate) async fn unsubscribe_from_topic(
        &self,
        tokens: &[String],
        topic: &str,
    ) -> Result<TopicManagementResponse, MessagingError> {
        self.batch_topic_operation(self.iid_endpoints.batch_remove(), tokens, topic)
            .await
    }

    /// Performs a topic subscribe/unsubscribe call, reporting per-token
    /// failures instead of failing the whole call when only some tokens are
    /// rejected — mirrors the official Admin SDKs'
    /// `mapRawResponseToTopicManagementResponse`, which walks every entry in
    /// `results[]` rather than treating any single error as fatal.
    async fn batch_topic_operation(
        &self,
        url: String,
        tokens: &[String],
        topic: &str,
    ) -> Result<TopicManagementResponse, MessagingError> {
        #[derive(serde::Serialize)]
        struct BatchTopicRequest<'a> {
            to: String,
            registration_tokens: &'a [String],
        }

        let topic = normalize_topic(topic);
        let response = self
            .post(&url)
            .json(&BatchTopicRequest {
                to: topic,
                registration_tokens: tokens,
            })
            .send()
            .await?;
        let parsed: IidResponse = parse_fcm_response(response).await?;

        let errors: Vec<TopicManagementError> = parsed
            .results
            .into_iter()
            .enumerate()
            .filter_map(|(index, result)| {
                result
                    .error
                    .map(|reason| TopicManagementError { index, reason })
            })
            .collect();

        Ok(TopicManagementResponse::from_errors(tokens.len(), errors))
    }
}

/// Prefixes a bare topic name with `/topics/`, as the Instance ID API
/// requires — mirrors the official Admin SDKs, which accept either form from
/// callers and normalize it before sending.
fn normalize_topic(topic: &str) -> String {
    if topic.starts_with("/topics/") {
        topic.to_string()
    } else {
        format!("/topics/{topic}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn operations_against(server: &MockServer) -> (HttpClient, FcmEndpoints, IidEndpoints) {
        (
            HttpClient::default(),
            FcmEndpoints::custom(server.uri()),
            IidEndpoints::custom(server.uri()),
        )
    }

    #[tokio::test]
    async fn send_returns_the_message_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/test-project/messages/0:1234567890"
            })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let message = Message::to_token("device-token");
        let message_id = ops.send(&message, false).await.unwrap();
        assert_eq!(message_id, "projects/test-project/messages/0:1234567890");
    }

    #[tokio::test]
    async fn send_sets_validate_only_on_dry_run() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .and(body_json(json!({
                "message": { "token": "device-token" },
                "validate_only": true,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "name": "msg-id" })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let message = Message::to_token("device-token");
        ops.send(&message, true).await.unwrap();
    }

    #[tokio::test]
    async fn send_surfaces_a_structured_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "error": {
                    "code": 404,
                    "message": "Requested entity was not found.",
                    "details": [{
                        "@type": "type.googleapis.com/google.firebase.fcm.v1.FcmError",
                        "errorCode": "UNREGISTERED",
                    }]
                }
            })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let message = Message::to_token("stale-token");
        let err = ops.send(&message, false).await.unwrap_err();
        match err {
            MessagingError::Api {
                status, error_code, ..
            } => {
                assert_eq!(status, 404);
                assert_eq!(error_code.as_deref(), Some("UNREGISTERED"));
            }
            other => panic!("expected MessagingError::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_falls_back_to_error_status_when_details_lack_an_error_code() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "error": {
                    "code": 403,
                    "message": "The caller does not have permission",
                    "status": "PERMISSION_DENIED",
                }
            })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let err = ops
            .send(&Message::to_token("some-token"), false)
            .await
            .unwrap_err();
        match err {
            MessagingError::Api { error_code, .. } => {
                assert_eq!(error_code.as_deref(), Some("PERMISSION_DENIED"));
            }
            other => panic!("expected MessagingError::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_for_batch_converts_api_errors_into_a_failure_result() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": { "message": "invalid", "details": [] }
            })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let message = Message::to_token("bad-token");
        let result = ops.send_for_batch(&message, false).await;
        assert!(matches!(result, SendResult::Failure { .. }));
    }

    #[tokio::test]
    async fn subscribe_to_topic_normalizes_a_bare_topic_name() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/iid/v1:batchAdd"))
            .and(body_json(json!({
                "to": "/topics/news",
                "registration_tokens": ["token-1", "token-2"],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{}, {}]
            })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let result = ops
            .subscribe_to_topic(&["token-1".to_string(), "token-2".to_string()], "news")
            .await
            .unwrap();
        assert_eq!(result.success_count, 2);
        assert_eq!(result.failure_count, 0);
    }

    #[tokio::test]
    async fn unsubscribe_from_topic_succeeds_on_empty_results() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/iid/v1:batchRemove"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{}]
            })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let result = ops
            .unsubscribe_from_topic(&["token-1".to_string()], "/topics/news")
            .await
            .unwrap();
        assert_eq!(result.success_count, 1);
        assert_eq!(result.failure_count, 0);
    }

    #[tokio::test]
    async fn subscribe_to_topic_reports_a_per_token_error_without_failing_the_call() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/iid/v1:batchAdd"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{}, { "error": "NOT_FOUND" }]
            })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        let result = ops
            .subscribe_to_topic(&["good-token".to_string(), "bad-token".to_string()], "news")
            .await
            .unwrap();
        assert_eq!(result.success_count, 1);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].index, 1);
        assert_eq!(result.errors[0].reason, "NOT_FOUND");
    }

    #[tokio::test]
    async fn post_sends_the_access_token_auth_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .and(wiremock::matchers::header("access_token_auth", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "name": "msg-id" })))
            .mount(&server)
            .await;

        let (http, fcm, iid) = operations_against(&server).await;
        let ops = MessagingOperations::new(&http, &fcm, &iid, "token");

        ops.send(&Message::to_token("device-token"), false)
            .await
            .unwrap();
    }
}
