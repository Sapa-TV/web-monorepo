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

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchUserSearchQuery {
    pub login: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct TwitchUserResponse {
    pub id: String,
    pub login: String,
    pub display_name: String,
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
    }))
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(start_twitch_auth))
        .routes(routes!(twitch_auth_callback))
        .routes(routes!(find_twitch_user))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::config::runtime::RuntimeConfig;
    use crate::config::static_config::StaticConfig;
    use crate::config::store::ConfigStore;
    use crate::config::twitch::TwitchConfig;
    use crate::db::inmemory_config::InMemoryConfigRepository;
    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::random::StandartRandomProvider;
    use crate::state::{AppState, AppStateBuilder};
    use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

    fn twitch_config() -> Arc<TwitchConfig> {
        Arc::new(TwitchConfig {
            client_id: "cid".to_string(),
            client_secret: "cs".to_string(),
            broadcaster_id: "bc".to_string(),
            redirect_uri: "https://localhost/cb".to_string(),
            credentials_redirect_uri: "https://localhost/creds/cb".to_string(),
            csrf_ttl_secs: 600,
        })
    }

    async fn state_with_twitch() -> AppState {
        let static_cfg = StaticConfig {
            twitch: Some(twitch_config()),
            ..StaticConfig::default()
        };
        let config_store = Arc::new(ConfigStore::new(
            Arc::new(static_cfg),
            RuntimeConfig::test_runtime("test-key"),
            Arc::new(InMemoryConfigRepository::new()),
        ));
        AppStateBuilder::new(
            StandartRandomProvider,
            config_store,
            Arc::new(InMemoryPlatformCredentialRepository::new()),
        )
        .with_empty_repos()
        .build()
        .await
        .expect("failed to build test state")
    }

    #[tokio::test]
    async fn find_user_requires_root_session() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/twitch/users?login=viewer"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn find_user_without_twitch_config_is_bad_request() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "100").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/twitch/users?login=viewer"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn find_user_without_credentials_is_unauthorized() {
        let state = state_with_twitch().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "100").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/twitch/users?login=viewer"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
