use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct IngressCredentialsResponse {
    pub configured: bool,
}

#[utoipa::path(
    get,
    path = "/admin/ingress/credentials",
    tag = "admin",
    responses(
        (status = 200, description = "Whether ingress credentials are configured", body = IngressCredentialsResponse),
    )
)]
pub async fn get_ingress_credentials(
    State(state): State<AppState>,
) -> Result<Json<IngressCredentialsResponse>, StatusCode> {
    let configured = state.admin_auth.is_ingress_credentials_configured().await?;
    Ok(Json(IngressCredentialsResponse { configured }))
}

#[utoipa::path(
    delete,
    path = "/admin/ingress/credentials",
    tag = "admin",
    responses(
        (status = 204, description = "Ingress credentials revoked"),
        (status = 500, description = "Failed to clear credentials"),
    )
)]
pub async fn revoke_ingress_credentials(
    State(state): State<AppState>,
) -> Result<StatusCode, StatusCode> {
    state.admin_auth.revoke_ingress_credentials().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_ingress_credentials, revoke_ingress_credentials))
}
