pub mod actions;
pub mod ingress;
pub mod orders;
pub mod rewards;
pub mod roulette;
pub mod rules;
pub mod twitch;
pub mod vk_video_live;

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
    #[serde(skip)]
    _sealed: (),
}

impl From<Admin> for AdminResponse {
    fn from(admin: Admin) -> Self {
        Self {
            twitch_id: admin.twitch_id,
            display_name: admin.display_name,
            is_root: admin.is_root,
            created_at: admin.created_at.to_rfc3339(),
            _sealed: (),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct AddAdminRequest {
    pub twitch_id: String,
    pub display_name: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct TwitchIdParam {
    pub twitch_id: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct WidgetAccessKeyResponse {
    pub widget_access_key: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct PresenceResponse {
    pub dock_connected: bool,
    pub widget_count: usize,
    #[serde(skip)]
    _sealed: (),
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
        _sealed: (),
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
    Ok(Json(WidgetAccessKeyResponse {
        widget_access_key,
        _sealed: (),
    }))
}

#[utoipa::path(
    get,
    path = "/admin/presence",
    tag = "admin",
    responses(
        (status = 200, description = "Current ws client presence", body = PresenceResponse),
    )
)]
pub async fn get_presence(State(state): State<AppState>) -> Json<PresenceResponse> {
    let snapshot = state.presence.snapshot();
    Json(PresenceResponse {
        dock_connected: snapshot.dock > 0,
        widget_count: snapshot.widget,
        _sealed: (),
    })
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_admins))
        .routes(routes!(get_widget_access_key))
        .routes(routes!(get_presence))
        .merge(actions::session_router())
        .merge(rules::session_router())
        .merge(rewards::session_router())
        .merge(roulette::session_router())
        .merge(orders::session_router())
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(add_admin))
        .routes(routes!(remove_admin))
        .routes(routes!(rotate_widget_access_key))
        .merge(twitch::root_router())
        .merge(vk_video_live::root_router())
        .merge(ingress::root_router())
        .merge(actions::root_router())
        .merge(rules::root_router())
}

#[cfg(test)]
#[path = "admin.test.rs"]
pub(crate) mod tests;
