use axum::Json;
use axum::extract::Extension;
use axum::extract::rejection::ExtensionRejection;
use axum::extract::{Query, State};
use axum::http::header;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::api::auth::{LOGIN_COOKIE, SESSION_COOKIE, cookie_header, read_cookie};
use crate::error::RepositoryError;
use crate::error::SessionServiceError;
use crate::session::Session;
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

#[utoipa::path(
    get,
    path = "/api/auth/twitch",
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
    path = "/api/auth/twitch/callback",
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
    Query(query): Query<TwitchLoginCallbackQuery>,
) -> Result<(StatusCode, HeaderMap, Json<TwitchLoginCallbackResponse>), StatusCode> {
    let exchanged = state
        .admin_auth
        .complete_login(&query.code, &query.state)
        .await?;
    let ticket = state
        .session_service
        .create_login_ticket(&exchanged.user_id, exchanged.user_name.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        cookie_header(
            LOGIN_COOKIE,
            ticket.ticket.as_str(),
            10 * 60,
            state.config.cookie_secure,
        ),
    );

    Ok((
        StatusCode::OK,
        headers,
        Json(TwitchLoginCallbackResponse {
            ticket: ticket.ticket.as_str().to_string(),
            twitch_user_id: exchanged.user_id,
            twitch_user_name: exchanged.user_name,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/sessions",
    tag = "auth",
    request_body = CreateSessionRequest,
    responses(
        (status = 201, description = "Session created, sapa_session cookie set", body = SessionResponse),
        (status = 400, description = "Invalid or already consumed login ticket"),
    )
)]
pub async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateSessionRequest>,
) -> Result<(StatusCode, HeaderMap, Json<SessionResponse>), SessionServiceError> {
    let login_cookie = read_cookie(&headers, LOGIN_COOKIE);
    if login_cookie.as_deref() != Some(body.ticket.as_str()) {
        return Err(SessionServiceError::InvalidTicket);
    }

    let ticket = state
        .session_service
        .consume_login_ticket(&body.ticket)
        .await?;

    let is_admin = state
        .admin_service
        .is_admin(&ticket.twitch_user_id)
        .await
        .map_err(|_| {
            SessionServiceError::Repo(RepositoryError::Database("admin service".to_string()))
        })?;
    if is_admin {
        state
            .admin_service
            .update_display_name(
                &ticket.twitch_user_id,
                ticket.twitch_user_name.as_deref().unwrap_or(""),
            )
            .await
            .ok();
    }

    let session = state
        .session_service
        .issue_session(&ticket.twitch_user_id, ticket.twitch_user_name.as_deref())
        .await?;

    let is_root = state
        .admin_service
        .is_root(&session.twitch_user_id)
        .await
        .map_err(|_| {
            SessionServiceError::Repo(RepositoryError::Database("admin service".to_string()))
        })?;

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        cookie_header(
            SESSION_COOKIE,
            session.token.as_str(),
            state.config.session_ttl_secs as i64,
            state.config.cookie_secure,
        ),
    );

    Ok((
        StatusCode::CREATED,
        headers,
        Json(SessionResponse {
            twitch_user_id: session.twitch_user_id.clone(),
            twitch_user_name: session.twitch_user_name.clone(),
            is_root,
            expires_at: session.expires_at.to_rfc3339(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/sessions/me",
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
    Ok(Json(SessionResponse {
        twitch_user_id: session.twitch_user_id.clone(),
        twitch_user_name: session.twitch_user_name.clone(),
        is_root,
        expires_at: session.expires_at.to_rfc3339(),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/sessions/me",
    tag = "auth",
    responses(
        (status = 204, description = "Session destroyed"),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<(StatusCode, HeaderMap), Unauthorized> {
    state
        .session_service
        .logout(session.token.as_str())
        .await
        .ok();
    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        cookie_header(SESSION_COOKIE, "", 0, state.config.cookie_secure),
    );
    Ok((StatusCode::NO_CONTENT, headers))
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
        .route("/api/auth/twitch", get(start_twitch_login))
        .route("/api/auth/twitch/callback", get(twitch_login_callback))
        .route("/api/sessions", post(create_session))
}

pub fn session_router() -> axum::Router<AppState> {
    use axum::routing::{delete, get};
    axum::Router::new()
        .route("/api/sessions/me", get(get_me))
        .route("/api/sessions/me", delete(logout))
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
    use crate::api::router_with_auth;
    use crate::state::AppState;
    use crate::test_fixtures::test_state;

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
        let app = router_with_auth(state.clone());
        let ticket = login_ticket(state, twitch_id).await;

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions")
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
    async fn create_session_returns_cookie_for_admin() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();

        let cookie = session_cookie(&state, "123").await;
        assert!(cookie.starts_with(&format!("{SESSION_COOKIE}=")));
    }

    #[tokio::test]
    async fn create_session_allows_regular_user() {
        let state = test_state().await;

        let response = create_session(&state, "999").await;
        assert_eq!(response.status(), StatusCode::CREATED);

        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["twitch_user_id"], "999");
        assert_eq!(body["is_root"], false);
    }

    #[tokio::test]
    async fn create_session_rejects_ticket_without_matching_cookie() {
        let state = test_state().await;
        let app = router_with_auth(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"ticket":"stolen-ticket"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn me_requires_session() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = router_with_auth(state.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/sessions/me")
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
                    .uri("/api/sessions/me")
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
        let app = router_with_auth(state.clone());

        let cookie = session_cookie(&state, "999").await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/sessions/me")
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
        let app = router_with_auth(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/sessions/me")
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
        let app = router_with_auth(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/sessions/me")
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
                    .uri("/api/sessions/me")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
