use axum::Json;
use axum::extract::Extension;
use axum::extract::rejection::ExtensionRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

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
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchLoginCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchLoginCallbackResponse {
    pub ticket: String,
    pub twitch_user_id: String,
    pub twitch_user_name: Option<String>,
}

impl From<(&ExchangedToken, &LoginTicket)> for TwitchLoginCallbackResponse {
    fn from((exchanged, ticket): (&ExchangedToken, &LoginTicket)) -> Self {
        Self {
            ticket: ticket.ticket.as_str().to_string(),
            twitch_user_id: exchanged.user_id.clone(),
            twitch_user_name: exchanged.user_name.clone(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct CreateSessionRequest {
    pub ticket: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct SessionResponse {
    pub twitch_user_id: String,
    pub twitch_user_name: Option<String>,
    pub is_root: bool,
    pub expires_at: String,
}

impl From<(&Session, bool)> for SessionResponse {
    fn from((session, is_root): (&Session, bool)) -> Self {
        Self {
            twitch_user_id: session.twitch_user_id.clone(),
            twitch_user_name: session.twitch_user_name.clone(),
            is_root,
            expires_at: session.expires_at.to_rfc3339(),
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
    Ok(Json(TwitchLoginStartResponse { auth_url }))
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

#[derive(OpenApi)]
#[openapi(
    paths(
        start_twitch_login,
        twitch_login_callback,
        create_session,
        get_me,
        logout
    ),
    components(schemas(
        TwitchLoginStartResponse,
        TwitchLoginCallbackResponse,
        CreateSessionRequest,
        SessionResponse,
    ))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct SessionApiDoc;

pub fn public_router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/auth/twitch", get(start_twitch_login))
        .route("/auth/twitch/callback", get(twitch_login_callback))
        .route("/sessions", post(create_session))
}

pub fn session_router() -> axum::Router<AppState> {
    use axum::routing::{delete, get};
    axum::Router::new()
        .route("/sessions/me", get(get_me))
        .route("/sessions/me", delete(logout))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use axum::response::Response;
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::auth::{LOGIN_COOKIE, SESSION_COOKIE};
    use crate::state::AppState;
    use crate::test_fixtures::{api_path, test_router, test_state};

    async fn login_ticket(state: &AppState, twitch_id: &str) -> String {
        state
            .session_service
            .create_login_ticket(twitch_id, Some("viewer"))
            .await
            .unwrap()
            .ticket
            .as_str()
            .to_string()
    }

    async fn create_session(state: &AppState, twitch_id: &str) -> Response {
        let app = test_router(state.clone());
        let ticket = login_ticket(state, twitch_id).await;

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/sessions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, format!("{LOGIN_COOKIE}={ticket}"))
                .body(Body::from(format!(r#"{{"ticket":"{ticket}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn session_cookie(state: &AppState, twitch_id: &str) -> String {
        let response = create_session(state, twitch_id).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn me_requires_session() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/sessions/me"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let cookie = session_cookie(&state, "123").await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/sessions/me"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["twitch_user_id"], "123");
    }

    #[tokio::test]
    async fn me_works_for_regular_user() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let cookie = session_cookie(&state, "999").await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/sessions/me"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_session_cookie_is_rejected() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/sessions/me"))
                    .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn logout_destroys_session() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path("/sessions/me"))
                    .header(header::COOKIE, cookie.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/sessions/me"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
