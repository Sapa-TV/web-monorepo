use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::actions::action::{Action, ActionId, ActionKind};
use crate::error::ActionServiceError;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ActionResponse {
    pub id: u32,
    pub name: String,
    pub kind: ActionKind,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Action> for ActionResponse {
    fn from(action: Action) -> Self {
        Self {
            id: action.id.get(),
            name: action.name,
            kind: action.kind,
            enabled: action.enabled,
            created_at: action.created_at.to_rfc3339(),
            updated_at: action.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertActionRequest {
    pub name: String,
    pub kind: ActionKind,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct ActionIdParam {
    pub id: u32,
}

#[utoipa::path(
    get,
    path = "/admin/actions",
    tag = "admin",
    responses(
        (status = 200, description = "List all actions", body = Vec<ActionResponse>),
    )
)]
pub async fn list_actions(
    State(state): State<AppState>,
) -> Result<Json<Vec<ActionResponse>>, ActionServiceError> {
    let actions = state.action_service.list().await?;
    Ok(Json(
        actions.into_iter().map(ActionResponse::from).collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/admin/actions",
    tag = "admin",
    request_body = UpsertActionRequest,
    responses(
        (status = 201, description = "Action created", body = ActionResponse),
    )
)]
pub async fn create_action(
    State(state): State<AppState>,
    Json(body): Json<UpsertActionRequest>,
) -> Result<(StatusCode, Json<ActionResponse>), ActionServiceError> {
    let action = state
        .action_service
        .create(&body.name, body.kind, body.enabled)
        .await?;
    Ok((StatusCode::CREATED, Json(ActionResponse::from(action))))
}

#[utoipa::path(
    put,
    path = "/admin/actions/{id}",
    tag = "admin",
    params(ActionIdParam),
    request_body = UpsertActionRequest,
    responses(
        (status = 200, description = "Action updated", body = ActionResponse),
        (status = 404, description = "Action not found"),
    )
)]
pub async fn update_action(
    State(state): State<AppState>,
    Path(param): Path<ActionIdParam>,
    Json(body): Json<UpsertActionRequest>,
) -> Result<Json<ActionResponse>, ActionServiceError> {
    let action = Action {
        id: ActionId::new(param.id),
        name: body.name,
        kind: body.kind,
        enabled: body.enabled,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    state.action_service.update(action).await?;
    let updated = state.action_service.get(ActionId::new(param.id)).await?;
    let updated = updated.ok_or(ActionServiceError::ActionNotFound)?;
    Ok(Json(ActionResponse::from(updated)))
}

#[utoipa::path(
    delete,
    path = "/admin/actions/{id}",
    tag = "admin",
    params(ActionIdParam),
    responses(
        (status = 204, description = "Action removed"),
        (status = 404, description = "Action not found"),
    )
)]
pub async fn delete_action(
    State(state): State<AppState>,
    Path(param): Path<ActionIdParam>,
) -> Result<StatusCode, ActionServiceError> {
    state.action_service.delete(ActionId::new(param.id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_actions))
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_action))
        .routes(routes!(update_action, delete_action))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::actions::action::ActionKind;
    use crate::api::auth::SESSION_COOKIE;
    use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

    fn action_body() -> &'static str {
        r#"{"name":"spin","kind":{"type":"enqueue_roulette"},"enabled":true}"#
    }

    #[tokio::test]
    async fn actions_require_session() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/actions"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_can_list_actions() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        state
            .action_service
            .create("spin", ActionKind::EnqueueRoulette, true)
            .await
            .unwrap();

        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/actions"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body.as_array().unwrap().len(), 1);
        assert_eq!(body[0]["name"], "spin");
        assert_eq!(body[0]["kind"]["type"], "enqueue_roulette");
    }

    #[tokio::test]
    async fn only_root_can_mutate_actions() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());

        let user_cookie = session_cookie(&state, "123").await;
        let root_cookie = session_cookie(&state, "100").await;

        let create = |cookie: String| {
            app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/actions"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie)
                    .body(Body::from(action_body()))
                    .unwrap(),
            )
        };

        let response = create(user_cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = create(root_cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(state.action_service.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn root_can_update_and_delete_action() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;
        let action = state
            .action_service
            .create("spin", ActionKind::EnqueueRoulette, true)
            .await
            .unwrap();
        let action_id = action.id.get();

        let updated = r#"{"name":"renamed","kind":{"type":"no_action"},"enabled":false}"#;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(api_path(&format!("/admin/actions/{action_id}")))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &root_cookie)
                    .body(Body::from(updated))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["name"], "renamed");
        assert_eq!(body["enabled"], false);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path(&format!("/admin/actions/{action_id}")))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.action_service.get(action.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_action_is_not_found() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path("/admin/actions/999"))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn actions_require_admin_session_cookie() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/actions"))
                    .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
