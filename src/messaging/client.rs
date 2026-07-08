//! The `MessagingClient` entry point and its builder.

use crate::core::{Credentials, HttpClient, ProjectId, ServiceAccountKey};
use crate::messaging::error::MessagingError;
use crate::messaging::fcm_v1::{FcmEndpoints, IidEndpoints, MessagingOperations};
use crate::messaging::message::{BatchResponse, Message, TopicManagementResponse};

/// Maximum number of messages accepted by a single [`MessagingClient::send_each`]
/// or [`MessagingClient::send_each_for_multicast`] call, matching the limit
/// documented for `sendEach`/`sendEachForMulticast` in the official Admin
/// SDKs.
pub const MAX_BATCH_SIZE: usize = 500;

/// Firebase Cloud Messaging client.
///
/// Unlike [`crate::auth::AuthClient`], there is no unauthenticated emulator
/// mode: every FCM v1 and Instance ID call requires a live OAuth2 bearer
/// token. Build one with [`MessagingClientBuilder`].
pub struct MessagingClient {
    http: HttpClient,
    #[cfg_attr(not(feature = "live-messaging"), allow(dead_code))]
    credentials: Credentials,
    fcm_endpoints: FcmEndpoints,
    iid_endpoints: IidEndpoints,
    legacy_http_transport: bool,
    #[cfg(feature = "live-messaging")]
    token_provider: tokio::sync::OnceCell<crate::messaging::token_provider::TokenProvider>,
}

impl MessagingClient {
    /// Starts building a new client for the given Firebase project.
    pub fn builder(project_id: impl Into<String>) -> MessagingClientBuilder {
        MessagingClientBuilder::new(project_id)
    }

    /// Sends a single message, returning FCM's assigned message id.
    ///
    /// When `dry_run` is `true`, the message is validated but not actually
    /// delivered (FCM v1's `validate_only`).
    pub async fn send(&self, message: &Message, dry_run: bool) -> Result<String, MessagingError> {
        let token = self.bearer_token().await?;
        self.operations(&token).send(message, dry_run).await
    }

    /// Sends up to [`MAX_BATCH_SIZE`] messages, returning a per-message
    /// success/failure result. Each message is sent as its own HTTP
    /// request, concurrently — mirroring the official Admin SDKs'
    /// `sendEach`/`sendEachForMulticast`, which dispatch every request
    /// before awaiting any of them (`Promise.allSettled`) rather than
    /// serializing round-trips. A failure sending one message does not
    /// prevent the others from being sent.
    pub async fn send_each(
        &self,
        messages: &[Message],
        dry_run: bool,
    ) -> Result<BatchResponse, MessagingError> {
        if messages.len() > MAX_BATCH_SIZE {
            return Err(MessagingError::BatchTooLarge {
                actual: messages.len(),
                max: MAX_BATCH_SIZE,
            });
        }
        let token = self.bearer_token().await?;
        let ops = self.operations(&token);

        let futures = messages
            .iter()
            .map(|message| ops.send_for_batch(message, dry_run));
        let results = futures_util::future::join_all(futures).await;
        Ok(BatchResponse::from_results(results))
    }

    /// Sends one message to up to [`MAX_BATCH_SIZE`] device registration
    /// tokens, individually. Equivalent to building one [`Message`] per
    /// token from a shared template and calling [`Self::send_each`].
    pub async fn send_each_for_multicast(
        &self,
        message_template: &Message,
        tokens: &[String],
        dry_run: bool,
    ) -> Result<BatchResponse, MessagingError> {
        let messages = multicast_messages(message_template, tokens);
        self.send_each(&messages, dry_run).await
    }

    /// Subscribes device registration tokens to a topic.
    ///
    /// A single call can partially fail: see [`TopicManagementResponse`].
    pub async fn subscribe_to_topic(
        &self,
        tokens: &[String],
        topic: &str,
    ) -> Result<TopicManagementResponse, MessagingError> {
        let token = self.bearer_token().await?;
        self.operations(&token)
            .subscribe_to_topic(tokens, topic)
            .await
    }

    /// Unsubscribes device registration tokens from a topic.
    ///
    /// A single call can partially fail: see [`TopicManagementResponse`].
    pub async fn unsubscribe_from_topic(
        &self,
        tokens: &[String],
        topic: &str,
    ) -> Result<TopicManagementResponse, MessagingError> {
        let token = self.bearer_token().await?;
        self.operations(&token)
            .unsubscribe_from_topic(tokens, topic)
            .await
    }

    /// Whether [`Self::send_each`]/[`Self::send_each_for_multicast`] were
    /// configured (via [`MessagingClientBuilder::enable_legacy_http_transport`])
    /// to send each message over its own HTTP/1.1 request instead of
    /// multiplexing over HTTP/2.
    ///
    /// This crate's HTTP client (`reqwest` over `hyper`) negotiates HTTP/2
    /// automatically when the server supports it and otherwise falls back to
    /// HTTP/1.1, so this flag is a no-op today; it exists so callers
    /// migrating from the official Admin SDKs (where this setting works
    /// around a legacy Node.js HTTP/2 bug) have an equivalent method to call
    /// without their code failing to compile.
    pub fn legacy_http_transport_enabled(&self) -> bool {
        self.legacy_http_transport
    }

    #[cfg(feature = "live-messaging")]
    async fn bearer_token(&self) -> Result<String, MessagingError> {
        let provider = self
            .token_provider
            .get_or_try_init(|| async {
                match &self.credentials {
                    Credentials::ServiceAccount(key) => {
                        crate::messaging::token_provider::TokenProvider::from_service_account(key)
                    }
                    Credentials::ApplicationDefault => {
                        crate::messaging::token_provider::TokenProvider::from_application_default()
                            .await
                    }
                    Credentials::Emulator => {
                        Err(MessagingError::Core(crate::core::CoreError::Credentials(
                            "FCM has no emulator mode; configure a service account key or \
                             Application Default Credentials"
                                .to_string(),
                        )))
                    }
                }
            })
            .await?;
        provider.access_token().await
    }

    #[cfg(not(feature = "live-messaging"))]
    async fn bearer_token(&self) -> Result<String, MessagingError> {
        Err(MessagingError::Core(crate::core::CoreError::Credentials(
            "sending FCM messages requires the `live-messaging` feature".to_string(),
        )))
    }

    fn operations<'a>(&'a self, bearer_token: &'a str) -> MessagingOperations<'a> {
        MessagingOperations::new(
            &self.http,
            &self.fcm_endpoints,
            &self.iid_endpoints,
            bearer_token,
        )
    }

    /// Builds a client pointed at a mock HTTP server for both the FCM v1 and
    /// Instance ID endpoints, with a fixed dummy bearer token so tests can
    /// exercise `send`/`send_each`/topic management against a `wiremock`
    /// server without resolving real OAuth2 credentials.
    #[cfg(test)]
    pub(crate) fn for_testing(base_url: &str) -> Self {
        Self {
            http: HttpClient::default(),
            credentials: Credentials::ServiceAccount(Box::new(ServiceAccountKey {
                client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
                private_key: String::new(),
                project_id: "test-project".to_string(),
                private_key_id: "test-key-id".to_string(),
            })),
            fcm_endpoints: FcmEndpoints::custom(base_url),
            iid_endpoints: IidEndpoints::custom(base_url),
            legacy_http_transport: false,
            #[cfg(feature = "live-messaging")]
            token_provider: tokio::sync::OnceCell::new(),
        }
    }

    /// Runs [`Self::send_each`] against the fixed test bearer token from
    /// [`Self::for_testing`], bypassing [`Self::bearer_token`]'s OAuth2
    /// resolution.
    #[cfg(test)]
    pub(crate) async fn send_each_for_testing(
        &self,
        messages: &[Message],
        dry_run: bool,
    ) -> BatchResponse {
        let ops = self.operations("test-bearer-token");
        let futures = messages
            .iter()
            .map(|message| ops.send_for_batch(message, dry_run));
        BatchResponse::from_results(futures_util::future::join_all(futures).await)
    }
}

/// Builds one [`Message`] per token from a shared template, for
/// [`MessagingClient::send_each_for_multicast`].
///
/// Batch-size validation is left to the [`MessagingClient::send_each`] call
/// this feeds into, so a too-large `tokens` slice surfaces as a recoverable
/// [`MessagingError::BatchTooLarge`] rather than being checked twice.
fn multicast_messages(message_template: &Message, tokens: &[String]) -> Vec<Message> {
    tokens
        .iter()
        .map(|token| {
            let mut m = message_template.clone();
            m.target = crate::messaging::message::Target::Token(token.clone());
            m
        })
        .collect()
}

/// Builds a [`MessagingClient`].
pub struct MessagingClientBuilder {
    project_id: String,
    service_account: Option<ServiceAccountKey>,
    #[cfg(feature = "live-messaging")]
    use_application_default_credentials: bool,
    legacy_http_transport: bool,
    http_client: Option<reqwest::Client>,
}

impl MessagingClientBuilder {
    /// Starts building a client for the given Firebase project id.
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            service_account: None,
            #[cfg(feature = "live-messaging")]
            use_application_default_credentials: false,
            legacy_http_transport: false,
            http_client: None,
        }
    }

    /// Authenticates using an explicit service account key.
    pub fn service_account_key(mut self, key: ServiceAccountKey) -> Self {
        self.service_account = Some(key);
        self
    }

    /// Authenticates using Application Default Credentials, resolved on
    /// first use: the `GOOGLE_APPLICATION_CREDENTIALS` environment
    /// variable, gcloud user credentials, or the GCE/Cloud Run metadata
    /// server, in that order (see [`gcp_auth::provider`]).
    #[cfg(feature = "live-messaging")]
    pub fn application_default_credentials(mut self) -> Self {
        self.use_application_default_credentials = true;
        self
    }

    /// Opts [`MessagingClient::send_each`]/[`MessagingClient::send_each_for_multicast`]
    /// into HTTP/1.1 transport instead of multiplexing over HTTP/2 — mirrors
    /// `Messaging.enableLegacyHttpTransport()` in the official Admin SDKs.
    /// See [`MessagingClient::legacy_http_transport_enabled`] for why this is
    /// a no-op on this crate's transport.
    pub fn enable_legacy_http_transport(mut self) -> Self {
        self.legacy_http_transport = true;
        self
    }

    /// Supplies a custom [`reqwest::Client`], e.g. for testing.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Builds the [`MessagingClient`].
    pub fn build(self) -> Result<MessagingClient, MessagingError> {
        let project_id = ProjectId::new(self.project_id).map_err(MessagingError::Core)?;

        let credentials = if let Some(key) = self.service_account {
            Credentials::ServiceAccount(Box::new(key))
        } else {
            #[cfg(feature = "live-messaging")]
            if self.use_application_default_credentials {
                Credentials::ApplicationDefault
            } else {
                return Err(MessagingError::Core(crate::core::CoreError::Credentials(
                    "no credentials configured: call service_account_key(...) or \
                     application_default_credentials()"
                        .to_string(),
                )));
            }
            #[cfg(not(feature = "live-messaging"))]
            return Err(MessagingError::Core(crate::core::CoreError::Credentials(
                "no credentials configured: call service_account_key(...)".to_string(),
            )));
        };

        let http = HttpClient::new(self.http_client.unwrap_or_default());
        let fcm_endpoints = FcmEndpoints::live(project_id.as_str());
        let iid_endpoints = IidEndpoints::live();

        Ok(MessagingClient {
            http,
            credentials,
            fcm_endpoints,
            iid_endpoints,
            legacy_http_transport: self.legacy_http_transport,
            #[cfg(feature = "live-messaging")]
            token_provider: tokio::sync::OnceCell::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::message::SendResult;
    use serde_json::json;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// `send_each` must dispatch all requests concurrently, not
    /// sequentially — mirroring the official Admin SDKs' `Promise.allSettled`
    /// behavior. Each mocked response is delayed by 200ms; 5 messages sent
    /// sequentially would take ~1s, while concurrent dispatch completes in
    /// roughly one delay period regardless of message count.
    #[tokio::test]
    async fn send_each_dispatches_requests_concurrently() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "name": "msg-id" }))
                    .set_delay(Duration::from_millis(200)),
            )
            .mount(&server)
            .await;

        let client = MessagingClient::for_testing(&server.uri());
        let messages: Vec<Message> = (0..5)
            .map(|i| Message::to_token(format!("token-{i}")))
            .collect();

        let started = std::time::Instant::now();
        let response = client.send_each_for_testing(&messages, false).await;
        let elapsed = started.elapsed();

        assert_eq!(response.success_count, 5);
        assert!(
            elapsed < Duration::from_millis(600),
            "send_each took {elapsed:?}, which suggests requests ran sequentially \
             (5 * 200ms delay) rather than concurrently"
        );
    }

    #[tokio::test]
    async fn send_each_for_multicast_targets_every_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/projects/test-project/messages:send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "name": "msg-id" })))
            .mount(&server)
            .await;

        let client = MessagingClient::for_testing(&server.uri());
        let template = Message::to_token("placeholder");
        let tokens = vec!["token-a".to_string(), "token-b".to_string()];

        let messages = multicast_messages(&template, &tokens);
        let response = client.send_each_for_testing(&messages, false).await;

        assert_eq!(response.success_count, 2);
        assert!(response
            .responses
            .iter()
            .all(|r| matches!(r, SendResult::Success { .. })));
    }

    #[test]
    fn builder_requires_credentials() {
        let result = MessagingClient::builder("some-project").build();
        match result {
            Err(MessagingError::Core(crate::core::CoreError::Credentials(_))) => {}
            _ => panic!("expected a Credentials error"),
        }
    }

    #[test]
    fn builder_rejects_an_empty_project_id() {
        let result = MessagingClient::builder("")
            .service_account_key(ServiceAccountKey {
                client_email: "test@test.iam.gserviceaccount.com".to_string(),
                private_key: String::new(),
                project_id: "test".to_string(),
                private_key_id: "key-1".to_string(),
            })
            .build();
        match result {
            Err(MessagingError::Core(crate::core::CoreError::InvalidProjectId(_))) => {}
            _ => panic!("expected an InvalidProjectId error"),
        }
    }

    #[tokio::test]
    async fn send_each_rejects_a_too_large_batch_without_panicking() {
        let client = MessagingClient::for_testing("http://localhost:0");
        let messages: Vec<Message> = (0..=MAX_BATCH_SIZE)
            .map(|i| Message::to_token(format!("token-{i}")))
            .collect();

        let result = client.send_each(&messages, true).await;
        match result {
            Err(MessagingError::BatchTooLarge { actual, max }) => {
                assert_eq!(actual, MAX_BATCH_SIZE + 1);
                assert_eq!(max, MAX_BATCH_SIZE);
            }
            other => panic!("expected BatchTooLarge, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_each_for_multicast_rejects_a_too_large_batch_without_panicking() {
        let client = MessagingClient::for_testing("http://localhost:0");
        let template = Message::to_token("placeholder");
        let tokens: Vec<String> = (0..=MAX_BATCH_SIZE).map(|i| format!("token-{i}")).collect();

        let result = client
            .send_each_for_multicast(&template, &tokens, true)
            .await;
        assert!(matches!(result, Err(MessagingError::BatchTooLarge { .. })));
    }
}
