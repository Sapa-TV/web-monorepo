use axum::Json;
use axum::extract::Extension;
use axum::extract::rejection::ExtensionRejection;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::auth::{LOGIN_COOKIE, SESSION_COOKIE, auth_cookie};
use crate::error::api::ApiError;
use composition::admin::auth::ExchangedToken;
use composition::state::AppState;
use model::consts::session;
use model::session::{LoginTicket, Session};

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct LoginStartResponse {
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
pub struct LoginCallbackResponse {
    pub ticket: String,
    pub platform: String,
    pub platform_user_id: String,
    pub platform_username: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl From<(&ExchangedToken, &LoginTicket)> for LoginCallbackResponse {
    fn from((exchanged, ticket): (&ExchangedToken, &LoginTicket)) -> Self {
        Self {
            ticket: ticket.ticket.as_str().to_string(),
            platform: exchanged.platform_id.name().to_string(),
            platform_user_id: exchanged.user_id.clone(),
            platform_username: exchanged.user_name.clone(),
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

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct LinkSessionRequest {
    pub ticket: String,
    #[serde(default)]
    pub merge: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct LinkedPlatformResponse {
    pub platform: String,
    pub platform_user_id: String,
    pub platform_username: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct SessionResponse {
    pub user_id: u32,
    pub platform: Option<String>,
    pub platform_user_id: Option<String>,
    pub platform_username: Option<String>,
    pub is_guest: bool,
    pub is_root: bool,
    pub expires_at: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<(&Session, bool)> for SessionResponse {
    fn from((session, is_root): (&Session, bool)) -> Self {
        Self {
            user_id: session.user_id.get(),
            platform: session.platform_id.map(|id| id.name().to_string()),
            platform_user_id: session.platform_user_id.clone(),
            platform_username: session.platform_username.clone(),
            is_guest: session.platform_user_id.is_none(),
            is_root,
            expires_at: session.expires_at.to_rfc3339(),
            _sealed: (),
        }
    }
}

fn cookie_max_age(session: &Session) -> i64 {
    (session.expires_at - Utc::now()).num_seconds().max(0)
}

fn guest_key(headers: &HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .unwrap_or("local")
        .trim()
        .chars()
        .take(64)
        .collect()
}

fn ensure_guest_allowed(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    state
        .guest_rate_limiter
        .allow(&guest_key(headers))
        .then_some(())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "too many guest sessions, try again later",
            )
        })
}

fn session_cookie_response(
    state: &AppState,
    jar: CookieJar,
    session: &Session,
    is_root: bool,
    status: StatusCode,
) -> (StatusCode, CookieJar, Json<SessionResponse>) {
    let cookie = auth_cookie(
        SESSION_COOKIE,
        session.token.as_str(),
        cookie_max_age(session),
        state.config.cookie_secure(),
    );
    (
        status,
        jar.add(cookie),
        Json(SessionResponse::from((session, is_root))),
    )
}

async fn is_root_session(state: &AppState, session: &Session) -> Result<bool, Unauthorized> {
    let twitch_id = state
        .account_service
        .twitch_user_id(session.user_id)
        .await
        .map_err(|_| Unauthorized)?;
    let Some(twitch_id) = twitch_id else {
        return Ok(false);
    };
    state
        .admin_service
        .is_root(&twitch_id)
        .await
        .map_err(|_| Unauthorized)
}

#[utoipa::path(
    get,
    path = "/auth/twitch",
    tag = "auth",
    responses(
        (status = 200, description = "Twitch login authorization URL", body = LoginStartResponse),
        (status = 400, description = "Twitch not configured"),
    )
)]
pub async fn start_twitch_login(
    State(state): State<AppState>,
) -> Result<Json<LoginStartResponse>, StatusCode> {
    let auth_url = state.admin_auth.start_login()?;
    Ok(Json(LoginStartResponse {
        auth_url,
        _sealed: (),
    }))
}

#[utoipa::path(
    get,
    path = "/auth/vk",
    tag = "auth",
    responses(
        (status = 200, description = "VK ID login authorization URL", body = LoginStartResponse),
        (status = 400, description = "VK ID not configured"),
    )
)]
pub async fn start_vk_login(
    State(state): State<AppState>,
) -> Result<Json<LoginStartResponse>, StatusCode> {
    let auth_url = state.vk_id_auth.start_login()?;
    Ok(Json(LoginStartResponse {
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
        (status = 200, description = "Identity exchanged, one-time login ticket issued", body = LoginCallbackResponse),
        (status = 400, description = "Twitch not configured"),
        (status = 403, description = "CSRF state mismatch or flow never started"),
    )
)]
pub async fn twitch_login_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<TwitchLoginCallbackQuery>,
) -> Result<(StatusCode, CookieJar, Json<LoginCallbackResponse>), ApiError> {
    let (exchanged, ticket) = state
        .session_service
        .exchange_login(&state.admin_auth, &query.code, &query.state)
        .await?;
    let response = LoginCallbackResponse::from((&exchanged, &ticket));
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
    get,
    path = "/auth/vk/callback",
    tag = "auth",
    params(TwitchLoginCallbackQuery),
    responses(
        (status = 200, description = "VK identity exchanged, one-time login ticket issued", body = LoginCallbackResponse),
        (status = 400, description = "VK ID not configured"),
        (status = 403, description = "CSRF state mismatch or flow never started"),
    )
)]
pub async fn vk_login_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<TwitchLoginCallbackQuery>,
) -> Result<(StatusCode, CookieJar, Json<LoginCallbackResponse>), ApiError> {
    let (exchanged, ticket) = state
        .session_service
        .exchange_login(&state.vk_id_auth, &query.code, &query.state)
        .await?;
    let response = LoginCallbackResponse::from((&exchanged, &ticket));
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
    path = "/sessions/guest",
    tag = "auth",
    responses(
        (status = 200, description = "Guest session, sapa_session cookie set", body = SessionResponse),
        (status = 429, description = "Too many guest sessions from this address"),
    )
)]
pub async fn create_guest_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<(StatusCode, CookieJar, Json<SessionResponse>), ApiError> {
    ensure_guest_allowed(&state, &headers)?;
    let current = jar.get(SESSION_COOKIE).map(|c| c.value().to_string());
    let (session, is_root) = state.account_service.guest(current.as_deref()).await?;
    Ok(session_cookie_response(
        &state,
        jar,
        &session,
        is_root,
        StatusCode::OK,
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
    let current = jar.get(SESSION_COOKIE).map(|c| c.value().to_string());
    let (session, is_root) = state
        .account_service
        .login(current.as_deref(), login_ticket.as_deref(), &body.ticket)
        .await?;
    Ok(session_cookie_response(
        &state,
        jar,
        &session,
        is_root,
        StatusCode::CREATED,
    ))
}

#[utoipa::path(
    post,
    path = "/sessions/link",
    tag = "auth",
    request_body = LinkSessionRequest,
    responses(
        (status = 200, description = "Provider linked to the current account", body = SessionResponse),
        (status = 401, description = "Not authenticated"),
        (status = 409, description = "Provider belongs to another account; retry with merge"),
    )
)]
pub async fn link_session(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LinkSessionRequest>,
) -> Result<(StatusCode, CookieJar, Json<SessionResponse>), ApiError> {
    let login_ticket = jar.get(LOGIN_COOKIE).map(|c| c.value().to_string());
    let current = jar.get(SESSION_COOKIE).map(|c| c.value().to_string());
    let (session, is_root) = state
        .account_service
        .link(
            current.as_deref(),
            login_ticket.as_deref(),
            &body.ticket,
            body.merge,
        )
        .await?;
    Ok(session_cookie_response(
        &state,
        jar,
        &session,
        is_root,
        StatusCode::OK,
    ))
}

#[utoipa::path(
    get,
    path = "/sessions/me/links",
    tag = "auth",
    responses(
        (status = 200, description = "Platforms linked to the current account", body = [LinkedPlatformResponse]),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn get_links(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Vec<LinkedPlatformResponse>>, Unauthorized> {
    let platforms = state
        .user_service
        .get_platforms(session.user_id)
        .await
        .map_err(|_| Unauthorized)?;
    Ok(Json(
        platforms
            .into_iter()
            .map(|platform| LinkedPlatformResponse {
                platform: platform.platform_id.name().to_string(),
                platform_user_id: platform.platform_user_id,
                platform_username: Some(platform.platform_username),
            })
            .collect(),
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
    let is_root = is_root_session(&state, &session).await?;
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
        .routes(routes!(start_vk_login))
        .routes(routes!(vk_login_callback))
        .routes(routes!(create_session))
        .routes(routes!(create_guest_session))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_me))
        .routes(routes!(get_links))
        .routes(routes!(link_session))
        .routes(routes!(logout))
}

#[cfg(test)]
#[path = "session.test.rs"]
mod tests;
