use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::OpenApi;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchAuthStartResponse {
    pub auth_url: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchAuthCallbackResponse {
    pub user_id: String,
    pub user_name: Option<String>,
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
    Ok(Json(TwitchAuthStartResponse { auth_url }))
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
    }))
}

#[derive(OpenApi)]
#[openapi(
    paths(start_twitch_auth, twitch_auth_callback),
    components(schemas(TwitchAuthStartResponse, TwitchAuthCallbackResponse,))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct AdminTwitchApiDoc;

pub fn root_router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/admin/twitch/auth", get(start_twitch_auth))
        .route("/admin/twitch/auth/callback", get(twitch_auth_callback))
}
