use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::error::api::ApiError;
use crate::platform::{Platform, PlatformId, PlatformRepository};
use crate::state::AppState;
use crate::user::repository::UserRepository;
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
        created_at: user.created_at.and_utc().to_rfc3339(),
        updated_at: user.updated_at.and_utc().to_rfc3339(),
    }
}

async fn build_user_response(
    user_id: UserId,
    platform_repo: &impl PlatformRepository,
    user_repo: &impl UserRepository,
) -> Result<UserResponse, ApiError> {
    let user = user_repo
        .get_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "user not found"))?;
    let user_platforms = user_repo.get_platforms(user_id).await?;
    let platforms = resolve_user_platforms(user_platforms, platform_repo).await?;
    Ok(to_user_response(user, platforms))
}

async fn resolve_user_platforms(
    user_platforms: Vec<UserPlatform>,
    platform_repo: &impl PlatformRepository,
) -> Result<Vec<UserPlatformResponse>, ApiError> {
    let all_platforms = platform_repo.load_all().await?;
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

async fn resolve_platform(
    name: &str,
    platform_repo: &impl PlatformRepository,
) -> Result<Platform, ApiError> {
    platform_repo
        .find_by_name(name)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, format!("unknown platform: {name}")))
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
    let user = state.user_repo.create(&body.display_name).await?;
    let response = build_user_response(user.id, &*state.platform_repo, &*state.user_repo).await?;
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
    let platform = resolve_platform(&query.platform, &*state.platform_repo).await?;
    let user = state
        .user_repo
        .find_by_platform(platform.id, &query.platform_user_id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "user not found"))?;
    let response = build_user_response(user.id, &*state.platform_repo, &*state.user_repo).await?;
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
    let response = build_user_response(params.id, &*state.platform_repo, &*state.user_repo).await?;
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
    let updated = state
        .user_repo
        .update_display_name(params.id, &body.display_name)
        .await?;
    if updated.is_none() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "user not found"));
    }
    let response = build_user_response(params.id, &*state.platform_repo, &*state.user_repo).await?;
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
    let deleted = state.user_repo.delete_user(params.id).await?;
    if !deleted {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "user not found"));
    }
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
    let platform = resolve_platform(&body.platform, &*state.platform_repo).await?;
    state
        .user_repo
        .link_platform(
            params.id,
            platform.id,
            &body.platform_user_id,
            &body.platform_username,
        )
        .await?;
    let response = build_user_response(params.id, &*state.platform_repo, &*state.user_repo).await?;
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
    let platform = resolve_platform(&params.platform, &*state.platform_repo).await?;
    let updated = state
        .user_repo
        .update_platform_username(params.id, platform.id, &body.platform_username)
        .await?;
    if updated.is_none() {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "platform link not found",
        ));
    }
    let response = build_user_response(params.id, &*state.platform_repo, &*state.user_repo).await?;
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
    let platform = resolve_platform(&params.platform, &*state.platform_repo).await?;
    let deleted = state
        .user_repo
        .delete_platform(params.id, platform.id)
        .await?;
    if !deleted {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "platform link not found",
        ));
    }
    let response = build_user_response(params.id, &*state.platform_repo, &*state.user_repo).await?;
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
    let platforms = state.platform_repo.load_all().await?;
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
pub(crate) struct UsersApiDoc;

pub fn router() -> axum::Router<AppState> {
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
    use axum::Router;
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::router;
    use crate::test_fixtures::test_state;
    use crate::user::repository::UserRepository;

    #[tokio::test]
    async fn create_user_201() {
        let state = test_state();
        let app = router(state.clone());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"display_name":"Viewer"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["display_name"], "Viewer");
        assert_eq!(body["id"], 1);
        assert!(body["created_at"].is_string());
    }

    #[tokio::test]
    async fn find_user_by_platform_200() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();
        state
            .user_repo
            .link_platform(
                user.id,
                crate::platform::PlatformId::new(1),
                "123",
                "twitch_user",
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=123")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["id"], user.id.value());
        assert_eq!(body["display_name"], "Viewer");
    }

    #[tokio::test]
    async fn find_user_by_platform_404() {
        let state = test_state();
        let app = router(state.clone());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=nonexistent")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn find_user_unknown_platform_400() {
        let state = test_state();
        let app = router(state.clone());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=unknown&platform_user_id=123")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn get_user_200() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri(&format!("/api/users/{}", user.id.value()))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn get_user_404() {
        let state = test_state();
        let app = router(state.clone());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/users/999")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn update_user_200() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("OldName").await.unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PATCH")
                    .uri(&format!("/api/users/{}", user.id.value()))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"display_name":"NewName"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["display_name"], "NewName");
    }

    #[tokio::test]
    async fn delete_user_204() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(&format!("/api/users/{}", user.id.value()))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 204);
    }

    #[tokio::test]
    async fn delete_user_404() {
        let state = test_state();
        let app = router(state.clone());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri("/api/users/999")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn link_platform_200() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(&format!("/api/users/{}/platforms", user.id.value()))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"platform":"twitch","platform_user_id":"123","platform_username":"tw_user"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["platforms"].as_array().unwrap().len(), 1);
        assert_eq!(body["platforms"][0]["platform"], "twitch");
    }

    #[tokio::test]
    async fn link_platform_409_duplicate() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("A").await.unwrap();

        state
            .user_repo
            .link_platform(user.id, crate::platform::PlatformId::new(1), "123", "user")
            .await
            .unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(&format!("/api/users/{}/platforms", user.id.value()))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
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
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(&format!("/api/users/{}/platforms", user.id.value()))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
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
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();
        state
            .user_repo
            .link_platform(
                user.id,
                crate::platform::PlatformId::new(1),
                "123",
                "old_name",
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PATCH")
                    .uri(&format!("/api/users/{}/platforms/twitch", user.id.value()))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"platform_username":"new_name"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["platforms"][0]["platform_username"], "new_name");
    }

    #[tokio::test]
    async fn delete_platform_200() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();
        state
            .user_repo
            .link_platform(user.id, crate::platform::PlatformId::new(1), "123", "user")
            .await
            .unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(&format!("/api/users/{}/platforms/twitch", user.id.value()))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(body["platforms"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_platform_404_nonexistent_link() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(&format!("/api/users/{}/platforms/twitch", user.id.value()))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn delete_platform_400_unknown_platform() {
        let state = test_state();
        let app = router(state.clone());

        let user = state.user_repo.create("Viewer").await.unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(&format!("/api/users/{}/platforms/unknown", user.id.value()))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn list_platforms_200() {
        let state = test_state();
        let app = router(state.clone());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/platforms")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body.as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn full_flow_new_viewer() {
        let state = test_state();
        let app = router(state.clone());

        // 1. Lookup — not found
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=123")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 404);

        // 2. Create user
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"display_name":"Viewer"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
        let user_body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let user_id = user_body["id"].as_u64().unwrap();

        // 3. Link platform
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(&format!("/api/users/{user_id}/platforms"))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
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
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/users?platform=twitch&platform_user_id=123")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }
}
