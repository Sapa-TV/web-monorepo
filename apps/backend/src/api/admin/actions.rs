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
    #[serde(skip)]
    _sealed: (),
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
            _sealed: (),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertActionRequest {
    pub name: String,
    pub kind: ActionKind,
    pub enabled: bool,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct ActionIdParam {
    pub id: u32,
    #[serde(skip)]
    _sealed: (),
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
    let action = Action::new(
        ActionId::new(param.id),
        body.name,
        body.kind,
        body.enabled,
        chrono::Utc::now(),
        chrono::Utc::now(),
    );
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
#[path = "actions.test.rs"]
mod tests;
