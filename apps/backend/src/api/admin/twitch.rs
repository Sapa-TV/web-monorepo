use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::OpenApi;
use utoipa::ToSchema;

use crate::admin::auth::AdminAuthError;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchAuthStartResponse {
    pub auth_url: String,
}

impl From<AdminAuthError> for StatusCode {
    fn from(e: AdminAuthError) -> Self {
        match e {
            AdminAuthError::NotConfigured => StatusCode::BAD_REQUEST,
            AdminAuthError::CsrfMismatch => StatusCode::FORBIDDEN,
            AdminAuthError::InvalidRedirectUri
            | AdminAuthError::Exchange
            | AdminAuthError::Persist => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
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
    path = "/api/admin/twitch/auth",
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
    path = "/api/admin/twitch/auth/callback",
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

pub fn protected_router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/admin/twitch/auth", get(start_twitch_auth))
        .route("/api/admin/twitch/auth/callback", get(twitch_auth_callback))
}
