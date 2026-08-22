pub mod actions;
pub mod ingress;
pub mod rewards;
pub mod roulette;
pub mod rules;
pub mod twitch;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin::Admin;
use crate::error::AdminServiceError;
use crate::error::api::ApiError;
use crate::session::Session;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct AdminResponse {
    pub twitch_id: String,
    pub display_name: Option<String>,
    pub is_root: bool,
    pub created_at: String,
}

impl From<Admin> for AdminResponse {
    fn from(admin: Admin) -> Self {
        Self {
            twitch_id: admin.twitch_id,
            display_name: admin.display_name,
            is_root: admin.is_root,
            created_at: admin.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct AddAdminRequest {
    pub twitch_id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchIdParam {
    pub twitch_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct WidgetAccessKeyResponse {
    pub widget_access_key: String,
}

#[utoipa::path(
    get,
    path = "/admin",
    tag = "admin",
    responses(
        (status = 200, description = "List all admins", body = Vec<AdminResponse>),
    )
)]
pub async fn list_admins(
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminResponse>>, AdminServiceError> {
    let admins = state.admin_service.list().await?;
    Ok(Json(admins.into_iter().map(AdminResponse::from).collect()))
}

#[utoipa::path(
    post,
    path = "/admin",
    tag = "admin",
    request_body = AddAdminRequest,
    responses(
        (status = 201, description = "Admin added", body = AdminResponse),
        (status = 409, description = "Admin already exists"),
    )
)]
pub async fn add_admin(
    State(state): State<AppState>,
    Json(body): Json<AddAdminRequest>,
) -> Result<(StatusCode, Json<AdminResponse>), AdminServiceError> {
    let admin = state
        .admin_service
        .add(&body.twitch_id, body.display_name.as_deref())
        .await?;
    Ok((StatusCode::CREATED, Json(AdminResponse::from(admin))))
}

#[utoipa::path(
    delete,
    path = "/admin/{twitch_id}",
    tag = "admin",
    params(TwitchIdParam),
    responses(
        (status = 204, description = "Admin removed"),
        (status = 404, description = "Admin not found"),
        (status = 403, description = "Cannot remove the last root admin"),
        (status = 409, description = "Cannot remove your own account"),
    )
)]
pub async fn remove_admin(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    Path(params): Path<TwitchIdParam>,
) -> Result<StatusCode, AdminServiceError> {
    if session.twitch_user_id == params.twitch_id {
        return Err(AdminServiceError::CannotRemoveSelf);
    }
    state.admin_service.remove(&params.twitch_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/admin/widget-access-key",
    tag = "admin",
    responses(
        (status = 200, description = "Widget access key", body = WidgetAccessKeyResponse),
    )
)]
pub async fn get_widget_access_key(State(state): State<AppState>) -> Json<WidgetAccessKeyResponse> {
    Json(WidgetAccessKeyResponse {
        widget_access_key: state.config.widget_access_key(),
    })
}

#[utoipa::path(
    post,
    path = "/admin/widget-access-key",
    tag = "admin",
    responses(
        (status = 200, description = "Widget access key rotated, new key generated", body = WidgetAccessKeyResponse),
    )
)]
pub async fn rotate_widget_access_key(
    State(state): State<AppState>,
) -> Result<Json<WidgetAccessKeyResponse>, ApiError> {
    let widget_access_key = state.config.rotate_widget_access_key_generated().await?;
    Ok(Json(WidgetAccessKeyResponse { widget_access_key }))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_admins))
        .routes(routes!(get_widget_access_key))
        .merge(actions::session_router())
        .merge(rules::session_router())
        .merge(rewards::session_router())
        .merge(roulette::session_router())
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(add_admin))
        .routes(routes!(remove_admin))
        .routes(routes!(rotate_widget_access_key))
        .merge(twitch::root_router())
        .merge(ingress::root_router())
        .merge(actions::root_router())
        .merge(rules::root_router())
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::auth::SESSION_COOKIE;
    use crate::config::repository::ConfigRepository;
    use crate::db::inmemory_config::InMemoryConfigRepository;
    use crate::db::inmemory_queue::InMemoryQueueRepository;
    use crate::test_fixtures::{
        api_path, session_cookie, test_router, test_state, test_state_with_data,
    };

    #[tokio::test]
    async fn admin_routes_require_session() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/widget-access-key"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn regular_user_is_forbidden_from_admin_routes() {
        let state = test_state().await;
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "999").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/widget-access-key"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_can_read_widget_access_key_and_list_admins() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/widget-access-key"))
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
        assert_eq!(body["widget_access_key"], "test-key");

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn only_root_can_add_admin() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());

        let user_cookie = session_cookie(&state, "123").await;
        let root_cookie = session_cookie(&state, "100").await;

        let add = |cookie: String| {
            app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie)
                    .body(Body::from(r#"{"twitch_id":"200","display_name":"mod"}"#))
                    .unwrap(),
            )
        };

        let response = add(user_cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = add(root_cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(state.admin_service.is_admin("200").await.unwrap());
    }

    #[tokio::test]
    async fn only_root_can_remove_admin() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        state.admin_service.add("200", None).await.unwrap();
        let app = test_router(state.clone());

        let user_cookie = session_cookie(&state, "123").await;
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path("/admin/200"))
                    .header(header::COOKIE, user_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path("/admin/200"))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(!state.admin_service.is_admin("200").await.unwrap());
    }

    #[tokio::test]
    async fn root_cannot_remove_self() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path("/admin/100"))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(state.admin_service.is_admin("100").await.unwrap());
    }

    #[tokio::test]
    async fn regular_session_cookie_is_rejected_by_admin_middleware() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/widget-access-key"))
                    .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn only_root_can_rotate_widget_access_key() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());

        let user_cookie = session_cookie(&state, "123").await;
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/widget-access-key"))
                    .header(header::COOKIE, user_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/widget-access-key"))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let widget_access_key = body["widget_access_key"].as_str().unwrap().to_string();
        assert!(!widget_access_key.is_empty());
        assert_ne!(widget_access_key, "test-key");
        assert_eq!(state.config.widget_access_key(), widget_access_key);
    }

    #[tokio::test]
    async fn rotate_widget_access_key_is_persisted_to_repo() {
        let config_repo = Arc::new(InMemoryConfigRepository::new());
        let state = test_state_with_data(
            Arc::new(InMemoryQueueRepository::new()),
            Arc::clone(&config_repo),
        )
        .await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/widget-access-key"))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let widget_access_key = body["widget_access_key"].as_str().unwrap().to_string();

        let stored = config_repo.load().await.unwrap().unwrap();
        assert_eq!(stored.widget_access_key, widget_access_key);
    }

    #[tokio::test]
    async fn rotated_widget_access_key_is_returned_by_get() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/widget-access-key"))
                    .header(header::COOKIE, &root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let widget_access_key = body["widget_access_key"].as_str().unwrap().to_string();
        assert_ne!(widget_access_key, "test-key");
        assert_eq!(state.config.widget_access_key(), widget_access_key);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/widget-access-key"))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["widget_access_key"], widget_access_key);
    }
}
