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
pub struct VkVideoLiveAuthStartResponse {
    pub auth_url: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct VkVideoLiveAuthCallbackQuery {
    pub code: String,
    pub state: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct VkVideoLiveAuthCallbackResponse {
    pub user_id: String,
    pub user_name: String,
    #[serde(skip)]
    _sealed: (),
}

#[utoipa::path(
    get,
    path = "/admin/vk-video-live/auth",
    tag = "admin",
    responses(
        (status = 200, description = "VK Video Live authorization URL", body = VkVideoLiveAuthStartResponse),
        (status = 400, description = "VK Video Live not configured"),
    )
)]
pub async fn start_vk_video_live_auth(
    State(state): State<AppState>,
) -> Result<Json<VkVideoLiveAuthStartResponse>, StatusCode> {
    let auth_url = state.vk_admin_auth.start()?;
    Ok(Json(VkVideoLiveAuthStartResponse {
        auth_url,
        _sealed: (),
    }))
}

#[utoipa::path(
    get,
    path = "/admin/vk-video-live/auth/callback",
    tag = "admin",
    params(VkVideoLiveAuthCallbackQuery),
    responses(
        (status = 200, description = "Token exchanged and credentials persisted", body = VkVideoLiveAuthCallbackResponse),
        (status = 400, description = "VK Video Live not configured"),
        (status = 403, description = "CSRF state mismatch or flow never started"),
    )
)]
pub async fn vk_video_live_auth_callback(
    State(state): State<AppState>,
    Query(query): Query<VkVideoLiveAuthCallbackQuery>,
) -> Result<Json<VkVideoLiveAuthCallbackResponse>, StatusCode> {
    let identity = state
        .vk_admin_auth
        .complete(&query.code, &query.state)
        .await?;
    Ok(Json(VkVideoLiveAuthCallbackResponse {
        user_id: identity.user_id,
        user_name: identity.user_name,
        _sealed: (),
    }))
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(start_vk_video_live_auth))
        .routes(routes!(vk_video_live_auth_callback))
}

#[cfg(test)]
#[path = "vk_video_live.test.rs"]
mod tests;
