//! User management operations against the Identity Toolkit REST API.

use crate::auth::error::{parse_identity_toolkit_response, AuthError};
use crate::auth::identity_toolkit::requests::{
    AccountsResponse, DeleteRequest, LookupRequest, SignUpRequest, UpdateRequest,
};
use crate::auth::identity_toolkit::IdentityToolkitEndpoints;
use crate::auth::users::model::{CreateUserRequest, UpdateUserRequest, UserRecord};
use crate::auth::users::query::UserPage;
use crate::core::{CoreError, HttpClient};

/// Performs user-management calls against the Identity Toolkit REST API.
///
/// Requires an OAuth2 bearer token (obtained from the configured service
/// account or Application Default Credentials) on every request except when
/// targeting the Firebase Auth Emulator, which instead requires a `key=`
/// query parameter to be present (any value; the emulator does not validate
/// it) — see [`crate::auth::mode::ClientMode::emulator_api_key`].
pub struct UserOperations<'a> {
    http: &'a HttpClient,
    endpoints: &'a IdentityToolkitEndpoints,
    bearer_token: Option<&'a str>,
    emulator_api_key: Option<&'static str>,
}

impl<'a> UserOperations<'a> {
    /// Creates a new set of user operations bound to the given transport
    /// and (when talking to production) bearer token, or (when talking to
    /// the emulator) dummy API key.
    pub fn new(
        http: &'a HttpClient,
        endpoints: &'a IdentityToolkitEndpoints,
        bearer_token: Option<&'a str>,
        emulator_api_key: Option<&'static str>,
    ) -> Self {
        Self {
            http,
            endpoints,
            bearer_token,
            emulator_api_key,
        }
    }

    fn request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut builder = self.http.inner().post(url);
        if let Some(token) = self.bearer_token {
            builder = builder.bearer_auth(token);
        }
        if let Some(key) = self.emulator_api_key {
            builder = builder.query(&[("key", key)]);
        }
        builder
    }

    /// Fetches a single user by uid.
    pub async fn get_user(&self, uid: &str) -> Result<UserRecord, AuthError> {
        let request = LookupRequest {
            local_id: vec![uid.to_string()],
            email: vec![],
        };
        let response = self
            .request(&self.endpoints.lookup())
            .json(&request)
            .send()
            .await?;
        let parsed: AccountsResponse = parse_identity_toolkit_response(response).await?;
        parsed
            .users
            .into_iter()
            .next()
            .map(UserRecord::from)
            .ok_or(AuthError::UserNotFound)
    }

    /// Fetches a single user by email address.
    pub async fn get_user_by_email(&self, email: &str) -> Result<UserRecord, AuthError> {
        let request = LookupRequest {
            local_id: vec![],
            email: vec![email.to_string()],
        };
        let response = self
            .request(&self.endpoints.lookup())
            .json(&request)
            .send()
            .await?;
        let parsed: AccountsResponse = parse_identity_toolkit_response(response).await?;
        parsed
            .users
            .into_iter()
            .next()
            .map(UserRecord::from)
            .ok_or(AuthError::UserNotFound)
    }

    /// Creates a new user.
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<UserRecord, AuthError> {
        let body = SignUpRequest {
            local_id: request.uid,
            email: request.email,
            password: request.password,
            display_name: request.display_name,
            disabled: request.disabled,
        };
        let response = self
            .request(&self.endpoints.sign_up())
            .json(&body)
            .send()
            .await?;
        let local_id: serde_json::Value = parse_identity_toolkit_response(response).await?;
        let uid = local_id
            .get("localId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AuthError::Api {
                status: 200,
                message: "signUp response missing localId".to_string(),
                error_code: None,
            })?;
        self.get_user(uid).await
    }

    /// Updates an existing user, including replacing its custom claims.
    pub async fn update_user(
        &self,
        uid: &str,
        request: UpdateUserRequest,
    ) -> Result<UserRecord, AuthError> {
        let custom_attributes = request
            .custom_claims
            .map(|claims| serde_json::to_string(&claims))
            .transpose()
            .map_err(CoreError::Deserialize)?;

        let body = UpdateRequest {
            local_id: uid.to_string(),
            email: request.email,
            display_name: request.display_name,
            disable_user: request.disabled,
            custom_attributes,
        };
        let response = self
            .request(&self.endpoints.update())
            .json(&body)
            .send()
            .await?;
        let _: serde_json::Value = parse_identity_toolkit_response(response).await?;
        self.get_user(uid).await
    }

    /// Replaces a user's custom claims entirely.
    pub async fn set_custom_user_claims(
        &self,
        uid: &str,
        claims: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), AuthError> {
        self.update_user(
            uid,
            UpdateUserRequest {
                custom_claims: Some(claims),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }

    /// Deletes a user by uid.
    pub async fn delete_user(&self, uid: &str) -> Result<(), AuthError> {
        let body = DeleteRequest {
            local_id: uid.to_string(),
        };
        let response = self
            .request(&self.endpoints.delete())
            .json(&body)
            .send()
            .await?;
        let _: serde_json::Value = parse_identity_toolkit_response(response).await?;
        Ok(())
    }

    /// Lists users, paginated via `next_page_token`.
    pub async fn list_users(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<UserPage, AuthError> {
        #[derive(serde::Serialize)]
        struct BatchGetQuery<'a> {
            #[serde(rename = "maxResults")]
            max_results: u32,
            #[serde(rename = "nextPageToken", skip_serializing_if = "Option::is_none")]
            next_page_token: Option<&'a str>,
        }

        let response = self
            .request(&self.endpoints.batch_get())
            .json(&BatchGetQuery {
                max_results,
                next_page_token: page_token,
            })
            .send()
            .await?;
        let parsed: AccountsResponse = parse_identity_toolkit_response(response).await?;

        Ok(UserPage {
            users: parsed.users.into_iter().map(UserRecord::from).collect(),
            next_page_token: parsed.next_page_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn operations_against(server: &MockServer) -> (HttpClient, IdentityToolkitEndpoints) {
        (
            HttpClient::default(),
            IdentityToolkitEndpoints::custom(server.uri()),
        )
    }

    #[tokio::test]
    async fn get_user_returns_the_matching_record() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/accounts:lookup"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "users": [{
                    "localId": "uid-1",
                    "email": "user@example.com",
                    "emailVerified": true,
                }]
            })))
            .mount(&server)
            .await;

        let (http, endpoints) = operations_against(&server).await;
        let ops = UserOperations::new(&http, &endpoints, Some("token"), None);

        let user = ops.get_user("uid-1").await.unwrap();
        assert_eq!(user.uid, "uid-1");
        assert_eq!(user.email.as_deref(), Some("user@example.com"));
        assert!(user.email_verified);
    }

    #[tokio::test]
    async fn get_user_with_no_matches_is_user_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/accounts:lookup"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "users": [] })))
            .mount(&server)
            .await;

        let (http, endpoints) = operations_against(&server).await;
        let ops = UserOperations::new(&http, &endpoints, Some("token"), None);

        let err = ops.get_user("missing-uid").await.unwrap_err();
        assert!(matches!(err, AuthError::UserNotFound));
    }

    #[tokio::test]
    async fn create_user_looks_up_the_new_user_after_sign_up() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/accounts:signUp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "localId": "new-uid" })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/accounts:lookup"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "users": [{ "localId": "new-uid", "email": "new@example.com" }]
            })))
            .mount(&server)
            .await;

        let (http, endpoints) = operations_against(&server).await;
        let ops = UserOperations::new(&http, &endpoints, Some("token"), None);

        let user = ops
            .create_user(CreateUserRequest {
                email: Some("new@example.com".to_string()),
                password: Some("correct horse battery staple".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(user.uid, "new-uid");
    }

    #[tokio::test]
    async fn delete_user_succeeds_on_a_200_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/accounts:delete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;

        let (http, endpoints) = operations_against(&server).await;
        let ops = UserOperations::new(&http, &endpoints, Some("token"), None);

        ops.delete_user("uid-1").await.unwrap();
    }

    #[tokio::test]
    async fn list_users_returns_records_and_next_page_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/accounts:batchGet"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "users": [
                    { "localId": "uid-1" },
                    { "localId": "uid-2" },
                ],
                "nextPageToken": "page-2",
            })))
            .mount(&server)
            .await;

        let (http, endpoints) = operations_against(&server).await;
        let ops = UserOperations::new(&http, &endpoints, Some("token"), None);

        let page = ops.list_users(10, None).await.unwrap();
        assert_eq!(page.users.len(), 2);
        assert_eq!(page.next_page_token.as_deref(), Some("page-2"));
    }

    #[tokio::test]
    async fn a_structured_api_error_populates_error_code() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/accounts:signUp"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": {
                    "code": 400,
                    "message": "EMAIL_EXISTS",
                    "errors": [{ "message": "EMAIL_EXISTS", "reason": "EMAIL_EXISTS" }]
                }
            })))
            .mount(&server)
            .await;

        let (http, endpoints) = operations_against(&server).await;
        let ops = UserOperations::new(&http, &endpoints, Some("token"), None);

        let err = ops
            .create_user(CreateUserRequest {
                email: Some("taken@example.com".to_string()),
                password: Some("correct horse battery staple".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err();

        match err {
            AuthError::Api {
                status, error_code, ..
            } => {
                assert_eq!(status, 400);
                assert_eq!(error_code.as_deref(), Some("EMAIL_EXISTS"));
            }
            other => panic!("expected AuthError::Api, got {other:?}"),
        }
    }
}
