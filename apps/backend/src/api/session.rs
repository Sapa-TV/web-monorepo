use axum::Json;
use axum::extract::Extension;
use axum::extract::rejection::ExtensionRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin::auth::ExchangedToken;
use crate::api::auth::{LOGIN_COOKIE, SESSION_COOKIE, auth_cookie};
use crate::consts::session;
use crate::error::api::ApiError;
use crate::session::{LoginTicket, Session};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchLoginStartResponse {
    pub auth_url: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchLoginCallbackQuery {
    pub code: String,
    pub state: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchLoginCallbackResponse {
    pub ticket: String,
    pub twitch_user_id: String,
    pub twitch_user_name: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl From<(&ExchangedToken, &LoginTicket)> for TwitchLoginCallbackResponse {
    fn from((exchanged, ticket): (&ExchangedToken, &LoginTicket)) -> Self {
        Self {
            ticket: ticket.ticket.as_str().to_string(),
            twitch_user_id: exchanged.user_id.clone(),
            twitch_user_name: exchanged.user_name.clone(),
            _sealed: (),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct CreateSessionRequest {
    pub ticket: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct SessionResponse {
    pub twitch_user_id: String,
    pub twitch_user_name: Option<String>,
    pub is_root: bool,
    pub expires_at: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<(&Session, bool)> for SessionResponse {
    fn from((session, is_root): (&Session, bool)) -> Self {
        Self {
            twitch_user_id: session.twitch_user_id.clone(),
            twitch_user_name: session.twitch_user_name.clone(),
            is_root,
            expires_at: session.expires_at.to_rfc3339(),
            _sealed: (),
        }
    }
}

#[utoipa::path(
    get,
    path = "/auth/twitch",
    tag = "auth",
    responses(
        (status = 200, description = "Twitch login authorization URL", body = TwitchLoginStartResponse),
        (status = 400, description = "Twitch not configured"),
    )
)]
pub async fn start_twitch_login(
    State(state): State<AppState>,
) -> Result<Json<TwitchLoginStartResponse>, StatusCode> {
    let auth_url = state.admin_auth.start_login()?;
    Ok(Json(TwitchLoginStartResponse {
        auth_url,
        _sealed: (),
    }))
}

#[utoipa::path(
    get,
    path = "/auth/twitch/callback",
    tag = "auth",
    params(TwitchLoginCallbackQuery),
    responses(
        (status = 200, description = "Identity exchanged, one-time login ticket issued", body = TwitchLoginCallbackResponse),
        (status = 400, description = "Twitch not configured"),
        (status = 403, description = "CSRF state mismatch or flow never started"),
    )
)]
pub async fn twitch_login_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<TwitchLoginCallbackQuery>,
) -> Result<(StatusCode, CookieJar, Json<TwitchLoginCallbackResponse>), ApiError> {
    let (exchanged, ticket) = state
        .session_service
        .exchange_login(&state.admin_auth, &query.code, &query.state)
        .await?;
    let response = TwitchLoginCallbackResponse::from((&exchanged, &ticket));
    Ok((
        StatusCode::OK,
        jar.add(auth_cookie(
            LOGIN_COOKIE,
            &response.ticket,
            session::LOGIN_TICKET_TTL.as_secs() as i64,
            state.config.cookie_secure(),
        )),
        Json(response),
    ))
}

#[utoipa::path(
    post,
    path = "/sessions",
    tag = "auth",
    request_body = CreateSessionRequest,
    responses(
        (status = 201, description = "Session created, sapa_session cookie set", body = SessionResponse),
        (status = 400, description = "Invalid or already consumed login ticket"),
    )
)]
pub async fn create_session(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<CreateSessionRequest>,
) -> Result<(StatusCode, CookieJar, Json<SessionResponse>), ApiError> {
    let login_ticket = jar.get(LOGIN_COOKIE).map(|c| c.value().to_string());
    let (session, is_root) = state
        .session_service
        .login(login_ticket.as_deref(), &body.ticket)
        .await?;
    let response = SessionResponse::from((&session, is_root));
    Ok((
        StatusCode::CREATED,
        jar.add(auth_cookie(
            SESSION_COOKIE,
            session.token.as_str(),
            state.config.session_ttl_secs() as i64,
            state.config.cookie_secure(),
        )),
        Json(response),
    ))
}

#[utoipa::path(
    get,
    path = "/sessions/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current session", body = SessionResponse),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<SessionResponse>, Unauthorized> {
    let is_root = state
        .admin_service
        .is_root(&session.twitch_user_id)
        .await
        .map_err(|_| Unauthorized)?;
    Ok(Json(SessionResponse::from((&session, is_root))))
}

#[utoipa::path(
    delete,
    path = "/sessions/me",
    tag = "auth",
    responses(
        (status = 204, description = "Session destroyed"),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    Extension(session): Extension<Session>,
) -> Result<(StatusCode, CookieJar), Unauthorized> {
    state
        .session_service
        .logout(session.token.as_str())
        .await
        .ok();
    let cookie = auth_cookie(SESSION_COOKIE, "", 0, state.config.cookie_secure());
    Ok((StatusCode::NO_CONTENT, jar.add(cookie)))
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Unauthorized;

impl From<ExtensionRejection> for Unauthorized {
    fn from(_: ExtensionRejection) -> Self {
        Unauthorized
    }
}

impl IntoResponse for Unauthorized {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, Json("unauthorized")).into_response()
    }
}

pub fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(start_twitch_login))
        .routes(routes!(twitch_login_callback))
        .routes(routes!(create_session))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_me, logout))
}

#[cfg(test)]
#[path = "session.test.rs"]
mod tests;
