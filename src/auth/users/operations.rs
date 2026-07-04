//! User management operations against the Identity Toolkit REST API.

use crate::auth::error::AuthError;
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
/// targeting the Firebase Auth Emulator.
pub struct UserOperations<'a> {
    http: &'a HttpClient,
    endpoints: &'a IdentityToolkitEndpoints,
    bearer_token: Option<&'a str>,
}

impl<'a> UserOperations<'a> {
    /// Creates a new set of user operations bound to the given transport
    /// and (when talking to production) bearer token.
    pub fn new(
        http: &'a HttpClient,
        endpoints: &'a IdentityToolkitEndpoints,
        bearer_token: Option<&'a str>,
    ) -> Self {
        Self {
            http,
            endpoints,
            bearer_token,
        }
    }

    fn request(&self, url: &str) -> reqwest::RequestBuilder {
        let builder = self.http.inner().post(url);
        match self.bearer_token {
            Some(token) => builder.bearer_auth(token),
            None => builder,
        }
    }

    async fn parse_response<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, AuthError> {
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(AuthError::Api {
                status,
                message,
                error_code: None,
            });
        }
        response
            .json::<T>()
            .await
            .map_err(|e| AuthError::Core(CoreError::Http(e)))
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
        let parsed: AccountsResponse = Self::parse_response(response).await?;
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
        let parsed: AccountsResponse = Self::parse_response(response).await?;
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
        let local_id: serde_json::Value = Self::parse_response(response).await?;
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
        let _: serde_json::Value = Self::parse_response(response).await?;
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
        let _: serde_json::Value = Self::parse_response(response).await?;
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
        let parsed: AccountsResponse = Self::parse_response(response).await?;

        Ok(UserPage {
            users: parsed.users.into_iter().map(UserRecord::from).collect(),
            next_page_token: parsed.next_page_token,
        })
    }
}
