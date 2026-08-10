use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::OpenApi;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct IngressCredentialsResponse {
    pub configured: bool,
}

#[utoipa::path(
    get,
    path = "/api/admin/ingress/credentials",
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
    path = "/api/admin/ingress/credentials",
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

#[derive(OpenApi)]
#[openapi(
    paths(get_ingress_credentials, revoke_ingress_credentials),
    components(schemas(IngressCredentialsResponse,))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct AdminIngressApiDoc;

pub fn root_router() -> axum::Router<AppState> {
    use axum::routing::{delete, get};
    axum::Router::new()
        .route(
            "/api/admin/ingress/credentials",
            get(get_ingress_credentials),
        )
        .route(
            "/api/admin/ingress/credentials",
            delete(revoke_ingress_credentials),
        )
}
