use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::error::api::ApiError;
use crate::platform::PlatformId;
use crate::state::AppState;
use crate::user::{UserId, UserPlatformId, UserView};

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

fn user_not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "user not found")
}

impl From<UserView> for UserResponse {
    fn from(view: UserView) -> Self {
        let UserView { user, platforms } = view;
        Self {
            id: user.id,
            display_name: user.display_name,
            platforms: platforms
                .into_iter()
                .map(|p| UserPlatformResponse {
                    id: p.id,
                    platform: p.platform_name,
                    platform_user_id: p.platform_user_id,
                    platform_username: p.platform_username,
                })
                .collect(),
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
        }
    }
}

#[utoipa::path(
    post,
    path = "/users",
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
    let view = state
        .user_service
        .build_user(user.id)
        .await?
        .ok_or_else(user_not_found)?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

#[utoipa::path(
    get,
    path = "/users",
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
        .ok_or_else(user_not_found)?;
    let view = state
        .user_service
        .build_user(user.id)
        .await?
        .ok_or_else(user_not_found)?;
    Ok(Json(view.into()))
}

#[utoipa::path(
    get,
    path = "/users/{id}",
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
    let view = state
        .user_service
        .build_user(params.id)
        .await?
        .ok_or_else(user_not_found)?;
    Ok(Json(view.into()))
}

#[utoipa::path(
    patch,
    path = "/users/{id}",
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
    let view = state
        .user_service
        .build_user(params.id)
        .await?
        .ok_or_else(user_not_found)?;
    Ok(Json(view.into()))
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
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
    path = "/users/{id}/platforms",
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
    let view = state
        .user_service
        .build_user(params.id)
        .await?
        .ok_or_else(user_not_found)?;
    Ok(Json(view.into()))
}

#[utoipa::path(
    patch,
    path = "/users/{id}/platforms/{platform}",
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
    let view = state
        .user_service
        .build_user(params.id)
        .await?
        .ok_or_else(user_not_found)?;
    Ok(Json(view.into()))
}

#[utoipa::path(
    delete,
    path = "/users/{id}/platforms/{platform}",
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
    let view = state
        .user_service
        .build_user(params.id)
        .await?
        .ok_or_else(user_not_found)?;
    Ok(Json(view.into()))
}

#[utoipa::path(
    get,
    path = "/platforms",
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

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{delete, get, patch, post};
    axum::Router::new()
        .route("/users", post(create_user))
        .route("/users", get(find_user))
        .route("/users/{id}", get(get_user))
        .route("/users/{id}", patch(update_user))
        .route("/users/{id}", delete(delete_user))
        .route("/users/{id}/platforms", post(link_platform))
        .route(
            "/users/{id}/platforms/{platform}",
            patch(update_platform_username),
        )
        .route("/users/{id}/platforms/{platform}", delete(delete_platform))
        .route("/platforms", get(list_platforms))
}
