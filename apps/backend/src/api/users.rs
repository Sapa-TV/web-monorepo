use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::error::api::ApiError;
use crate::platform::PlatformId;
use crate::state::AppState;
use crate::user::{User, UserId, UserPlatform, UserPlatformId};

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct UserResponse {
    pub id: UserId,
    pub display_name: String,
    pub platforms: Vec<UserPlatformResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct UserPlatformResponse {
    pub id: UserPlatformId,
    pub platform: String,
    pub platform_user_id: String,
    pub platform_username: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct PlatformResponse {
    pub id: PlatformId,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct CreateUserRequest {
    pub display_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct LinkPlatformRequest {
    pub platform: String,
    pub platform_user_id: String,
    pub platform_username: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpdateUserRequest {
    pub display_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpdatePlatformRequest {
    pub platform_username: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct FindUserQuery {
    pub platform: String,
    pub platform_user_id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct UserIdParam {
    pub id: UserId,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct PlatformNameParam {
    pub id: UserId,
    pub platform: String,
}

fn to_user_response(user: User, platforms: Vec<UserPlatformResponse>) -> UserResponse {
    UserResponse {
        id: user.id,
        display_name: user.display_name,
        platforms,
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    }
}

async fn build_user_response(state: &AppState, user_id: UserId) -> Result<UserResponse, ApiError> {
    let user = state
        .user_service
        .get_user(user_id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "user not found"))?;
    let user_platforms = state.user_service.get_platforms(user_id).await?;
    let platforms = resolve_user_platforms(user_platforms, state).await?;
    Ok(to_user_response(user, platforms))
}

async fn resolve_user_platforms(
    user_platforms: Vec<UserPlatform>,
    state: &AppState,
) -> Result<Vec<UserPlatformResponse>, ApiError> {
    let all_platforms = state.user_service.list_platforms().await?;
    let mut result = Vec::with_capacity(user_platforms.len());
    for up in user_platforms {
        let platform_name = all_platforms
            .iter()
            .find(|p| p.id == up.platform_id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        result.push(UserPlatformResponse {
            id: up.id,
            platform: platform_name,
            platform_user_id: up.platform_user_id,
            platform_username: up.platform_username,
        });
    }
    Ok(result)
}

#[utoipa::path(
    post,
    path = "/api/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = UserResponse),
    )
)]
pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let user = state.user_service.create(&body.display_name).await?;
    let response = build_user_response(&state, user.id).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/users",
    tag = "users",
    params(FindUserQuery),
    responses(
        (status = 200, description = "User found", body = UserResponse),
        (status = 404, description = "User not found"),
        (status = 400, description = "Unknown platform"),
    )
)]
pub async fn find_user(
    State(state): State<AppState>,
    Query(query): Query<FindUserQuery>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state
        .user_service
        .find_by_platform(&query.platform, &query.platform_user_id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "user not found"))?;
    let response = build_user_response(&state, user.id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "users",
    params(UserIdParam),
    responses(
        (status = 200, description = "User found", body = UserResponse),
        (status = 404, description = "User not found"),
    )
)]
pub async fn get_user(
    State(state): State<AppState>,
    Path(params): Path<UserIdParam>,
) -> Result<Json<UserResponse>, ApiError> {
    let response = build_user_response(&state, params.id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    patch,
    path = "/api/users/{id}",
    tag = "users",
    params(UserIdParam),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 404, description = "User not found"),
    )
)]
pub async fn update_user(
    State(state): State<AppState>,
    Path(params): Path<UserIdParam>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    state
        .user_service
        .update_user(params.id, &body.display_name)
        .await?;
    let response = build_user_response(&state, params.id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    tag = "users",
    params(UserIdParam),
    responses(
        (status = 204, description = "User deleted"),
        (status = 404, description = "User not found"),
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    Path(params): Path<UserIdParam>,
) -> Result<StatusCode, ApiError> {
    state.user_service.delete_user(params.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/users/{id}/platforms",
    tag = "users",
    params(UserIdParam),
    request_body = LinkPlatformRequest,
    responses(
        (status = 200, description = "Platform linked", body = UserResponse),
        (status = 400, description = "Unknown platform"),
        (status = 404, description = "User not found"),
        (status = 409, description = "Platform already linked"),
    )
)]
pub async fn link_platform(
    State(state): State<AppState>,
    Path(params): Path<UserIdParam>,
    Json(body): Json<LinkPlatformRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    state
        .user_service
        .link_platform(
            params.id,
            &body.platform,
            &body.platform_user_id,
            &body.platform_username,
        )
        .await?;
    let response = build_user_response(&state, params.id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    patch,
    path = "/api/users/{id}/platforms/{platform}",
    tag = "users",
    params(PlatformNameParam),
    request_body = UpdatePlatformRequest,
    responses(
        (status = 200, description = "Platform username updated", body = UserResponse),
        (status = 400, description = "Unknown platform"),
        (status = 404, description = "User or platform link not found"),
    )
)]
pub async fn update_platform_username(
    State(state): State<AppState>,
    Path(params): Path<PlatformNameParam>,
    Json(body): Json<UpdatePlatformRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    state
        .user_service
        .update_platform_username(params.id, &params.platform, &body.platform_username)
        .await?;
    let response = build_user_response(&state, params.id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}/platforms/{platform}",
    tag = "users",
    params(PlatformNameParam),
    responses(
        (status = 200, description = "Platform unlinked", body = UserResponse),
        (status = 400, description = "Unknown platform"),
        (status = 404, description = "User or platform link not found"),
    )
)]
pub async fn delete_platform(
    State(state): State<AppState>,
    Path(params): Path<PlatformNameParam>,
) -> Result<Json<UserResponse>, ApiError> {
    state
        .user_service
        .delete_platform(params.id, &params.platform)
        .await?;
    let response = build_user_response(&state, params.id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/platforms",
    tag = "users",
    responses(
        (status = 200, description = "List all platforms", body = Vec<PlatformResponse>),
    )
)]
pub async fn list_platforms(
    State(state): State<AppState>,
) -> Result<Json<Vec<PlatformResponse>>, ApiError> {
    let platforms = state.user_service.list_platforms().await?;
    Ok(Json(
        platforms
            .into_iter()
            .map(|p| PlatformResponse {
                id: p.id,
                name: p.name,
            })
            .collect(),
    ))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        create_user,
        find_user,
        get_user,
        update_user,
        delete_user,
        link_platform,
        update_platform_username,
        delete_platform,
        list_platforms,
    ),
    components(schemas(
        UserResponse,
        UserPlatformResponse,
        PlatformResponse,
        CreateUserRequest,
        LinkPlatformRequest,
        UpdateUserRequest,
        UpdatePlatformRequest,
    ))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct UsersApiDoc;

pub fn protected_router() -> axum::Router<AppState> {
    use axum::routing::{delete, get, patch, post};
    axum::Router::new()
        .route("/api/users", post(create_user))
        .route("/api/users", get(find_user))
        .route("/api/users/{id}", get(get_user))
        .route("/api/users/{id}", patch(update_user))
        .route("/api/users/{id}", delete(delete_user))
        .route("/api/users/{id}/platforms", post(link_platform))
        .route(
            "/api/users/{id}/platforms/{platform}",
            patch(update_platform_username),
        )
        .route(
            "/api/users/{id}/platforms/{platform}",
            delete(delete_platform),
        )
        .route("/api/platforms", get(list_platforms))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::Request;
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::router;
    use crate::test_fixtures::test_state;

    #[tokio::test]
    async fn create_user_201() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"display_name":"Viewer"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["display_name"], "Viewer");
        assert!(body["id"].as_u64().is_some());
        assert!(body["created_at"].is_string());
    }

    #[tokio::test]
    async fn find_user_by_platform_200() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();
        state
            .user_service
            .link_platform(user.id, "twitch", "123", "twitch_user")
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["id"], serde_json::to_value(user.id).unwrap());
        assert_eq!(body["display_name"], "Viewer");
    }

    #[tokio::test]
    async fn find_user_by_platform_404() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn find_user_unknown_platform_400() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=unknown&platform_user_id=123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn get_user_200() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/users/{}", user.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["display_name"], "Viewer");
    }

    #[tokio::test]
    async fn get_user_404() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/users/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn update_user_200() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("OldName").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/users/{}", user.id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"display_name":"NewName"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["display_name"], "NewName");
    }

    #[tokio::test]
    async fn update_user_404() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/users/999")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"display_name":"NewName"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn delete_user_204() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{}", user.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 204);
    }

    #[tokio::test]
    async fn delete_user_404() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/users/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn link_platform_200() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/users/{}/platforms", user.id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"platform":"twitch","platform_user_id":"123","platform_username":"tw_user"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["platforms"].as_array().unwrap().len(), 1);
        assert_eq!(body["platforms"][0]["platform"], "twitch");
    }

    #[tokio::test]
    async fn link_platform_404_nonexistent_user() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/users/999/platforms")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"platform":"twitch","platform_user_id":"123","platform_username":"u"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn link_platform_409_duplicate() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("A").await.unwrap();

        state
            .user_service
            .link_platform(user.id, "twitch", "123", "user")
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/users/{}/platforms", user.id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"platform":"twitch","platform_user_id":"123","platform_username":"other"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 409);
    }

    #[tokio::test]
    async fn link_platform_400_unknown_platform() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/users/{}/platforms", user.id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"platform":"unknown","platform_user_id":"123","platform_username":"u"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn update_platform_username_200() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();
        state
            .user_service
            .link_platform(user.id, "twitch", "123", "old_name")
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/users/{}/platforms/twitch", user.id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"platform_username":"new_name"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["platforms"][0]["platform_username"], "new_name");
    }

    #[tokio::test]
    async fn update_platform_username_404_missing_link() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/users/{}/platforms/twitch", user.id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"platform_username":"new_name"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn delete_platform_200() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();
        state
            .user_service
            .link_platform(user.id, "twitch", "123", "user")
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{}/platforms/twitch", user.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert!(body["platforms"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_platform_404_nonexistent_link() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{}/platforms/twitch", user.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn delete_platform_400_unknown_platform() {
        let state = test_state().await;
        let app = router(state.clone());

        let user = state.user_service.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{}/platforms/unknown", user.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn list_platforms_200() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/platforms")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body.as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn full_flow_new_viewer() {
        let state = test_state().await;
        let app = router(state.clone());

        // 1. Lookup — not found
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 404);

        // 2. Create user
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"display_name":"Viewer"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
        let user_body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let user_id = user_body["id"].as_u64().unwrap();

        // 3. Link platform
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/users/{user_id}/platforms"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"platform":"twitch","platform_user_id":"123","platform_username":"tw_user"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);

        // 4. Lookup — found now
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }
}
