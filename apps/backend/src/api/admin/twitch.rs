use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchAuthStartResponse {
    pub auth_url: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchAuthCallbackQuery {
    pub code: String,
    pub state: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchAuthCallbackResponse {
    pub user_id: String,
    pub user_name: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchUserSearchQuery {
    pub login: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchUserResponse {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(skip)]
    _sealed: (),
}

#[utoipa::path(
    get,
    path = "/admin/twitch/auth",
    tag = "admin",
    responses(
        (status = 200, description = "Twitch authorization URL", body = TwitchAuthStartResponse),
        (status = 400, description = "Twitch not configured"),
    )
)]
pub async fn start_twitch_auth(
    State(state): State<AppState>,
) -> Result<Json<TwitchAuthStartResponse>, StatusCode> {
    let auth_url = state.admin_auth.start()?;
    Ok(Json(TwitchAuthStartResponse {
        auth_url,
        _sealed: (),
    }))
}

#[utoipa::path(
    get,
    path = "/admin/twitch/auth/callback",
    tag = "admin",
    params(TwitchAuthCallbackQuery),
    responses(
        (status = 200, description = "Token exchanged and refresh token persisted", body = TwitchAuthCallbackResponse),
        (status = 400, description = "Twitch not configured"),
        (status = 403, description = "CSRF state mismatch or flow never started"),
    )
)]
pub async fn twitch_auth_callback(
    State(state): State<AppState>,
    Query(query): Query<TwitchAuthCallbackQuery>,
) -> Result<Json<TwitchAuthCallbackResponse>, StatusCode> {
    let exchanged = state.admin_auth.complete(&query.code, &query.state).await?;
    Ok(Json(TwitchAuthCallbackResponse {
        user_id: exchanged.user_id,
        user_name: exchanged.user_name,
        _sealed: (),
    }))
}

#[utoipa::path(
    get,
    path = "/admin/twitch/users",
    tag = "admin",
    params(TwitchUserSearchQuery),
    responses(
        (status = 200, description = "Twitch user by login", body = TwitchUserResponse),
        (status = 400, description = "Twitch is not configured"),
        (status = 404, description = "Twitch user not found"),
    )
)]
pub async fn find_twitch_user(
    State(state): State<AppState>,
    Query(query): Query<TwitchUserSearchQuery>,
) -> Result<Json<TwitchUserResponse>, StatusCode> {
    let twitch = state.twitch_api.as_ref().ok_or(StatusCode::BAD_REQUEST)?;
    let user = twitch.find_user_by_login(&query.login).await?;
    let user = user.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(TwitchUserResponse {
        id: user.id.to_string(),
        login: user.login.to_string(),
        display_name: user.display_name.to_string(),
        _sealed: (),
    }))
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(start_twitch_auth))
        .routes(routes!(twitch_auth_callback))
        .routes(routes!(find_twitch_user))
}

#[cfg(test)]
#[path = "twitch.test.rs"]
mod tests;
