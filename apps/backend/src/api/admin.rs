pub mod ingress;
pub mod twitch;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::admin::Admin;
use crate::error::AdminServiceError;
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
pub struct PakResponse {
    pub pak: String,
}

#[utoipa::path(
    get,
    path = "/api/admin",
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
    path = "/api/admin",
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
    path = "/api/admin/{twitch_id}",
    tag = "admin",
    params(TwitchIdParam),
    responses(
        (status = 204, description = "Admin removed"),
        (status = 404, description = "Admin not found"),
        (status = 403, description = "Cannot remove the last root admin"),
    )
)]
pub async fn remove_admin(
    State(state): State<AppState>,
    Path(params): Path<TwitchIdParam>,
) -> Result<StatusCode, AdminServiceError> {
    state.admin_service.remove(&params.twitch_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/admin/pak",
    tag = "admin",
    responses(
        (status = 200, description = "Widget access key (PAK)", body = PakResponse),
    )
)]
pub async fn get_admin_pak(State(state): State<AppState>) -> Json<PakResponse> {
    Json(PakResponse {
        pak: state.config.access_key(),
    })
}

#[derive(OpenApi)]
#[openapi(
    paths(list_admins, add_admin, remove_admin, get_admin_pak),
    components(schemas(AdminResponse, AddAdminRequest, PakResponse,))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct AdminApiDoc;

pub fn session_router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/admin", get(list_admins))
        .route("/api/admin/pak", get(get_admin_pak))
}

pub fn root_router() -> axum::Router<AppState> {
    use axum::routing::{delete, post};
    axum::Router::new()
        .route("/api/admin", post(add_admin))
        .route("/api/admin/{twitch_id}", delete(remove_admin))
        .merge(twitch::root_router())
        .merge(ingress::root_router())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::auth::{LOGIN_COOKIE, SESSION_COOKIE};
    use crate::api::router_with_auth;
    use crate::state::AppState;
    use crate::test_fixtures::test_state;

    async fn session_cookie(state: &AppState, twitch_id: &str) -> String {
        let app = router_with_auth(state.clone());
        let ticket = state
            .session_service
            .create_login_ticket(twitch_id, Some("viewer"))
            .await
            .unwrap()
            .ticket
            .as_str()
            .to_string();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, format!("{LOGIN_COOKIE}={ticket}"))
                    .body(Body::from(format!(r#"{{"ticket":"{ticket}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
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
    async fn admin_routes_require_session() {
        let state = test_state().await;
        let app = router_with_auth(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/admin/pak")
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
        let app = router_with_auth(state.clone());
        let cookie = session_cookie(&state, "999").await;

        for uri in ["/api/admin/pak", "/api/admin"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(uri)
                        .header(header::COOKIE, &cookie)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "uri: {uri}");
        }
    }

    #[tokio::test]
    async fn admin_can_read_pak_and_list_admins() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = router_with_auth(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/admin/pak")
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
        assert_eq!(body["pak"], "test-key");

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/admin")
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
        let app = router_with_auth(state.clone());

        let user_cookie = session_cookie(&state, "123").await;
        let root_cookie = session_cookie(&state, "100").await;

        let add = |cookie: String| {
            app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin")
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
        let app = router_with_auth(state.clone());

        let user_cookie = session_cookie(&state, "123").await;
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/admin/200")
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
                    .uri("/api/admin/200")
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
    async fn regular_session_cookie_is_rejected_by_admin_middleware() {
        let state = test_state().await;
        let app = router_with_auth(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/admin/pak")
                    .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
