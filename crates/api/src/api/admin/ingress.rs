use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use composition::ingress::credentials::IngressPlatform;
use composition::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct IngressCredentialsResponse {
    pub twitch: bool,
    pub vk_video_live: bool,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct IngressPlatformQuery {
    #[serde(default)]
    pub platform: IngressPlatform,
    #[serde(skip)]
    _sealed: (),
}

#[utoipa::path(
    get,
    path = "/admin/ingress/credentials",
    tag = "admin",
    responses(
        (status = 200, description = "Whether ingress credentials are configured per platform", body = IngressCredentialsResponse),
    )
)]
pub async fn get_ingress_credentials(
    State(state): State<AppState>,
) -> Result<Json<IngressCredentialsResponse>, StatusCode> {
    Ok(Json(IngressCredentialsResponse {
        twitch: IngressPlatform::Twitch.credentials_configured(&state).await,
        vk_video_live: IngressPlatform::VkVideoLive
            .credentials_configured(&state)
            .await,
        _sealed: (),
    }))
}

#[utoipa::path(
    delete,
    path = "/admin/ingress/credentials",
    tag = "admin",
    params(IngressPlatformQuery),
    responses(
        (status = 204, description = "Ingress credentials revoked"),
        (status = 400, description = "Platform not configured"),
        (status = 500, description = "Failed to clear credentials"),
    )
)]
pub async fn revoke_ingress_credentials(
    State(state): State<AppState>,
    Query(query): Query<IngressPlatformQuery>,
) -> Result<StatusCode, StatusCode> {
    query.platform.revoke_credentials(&state).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_ingress_credentials, revoke_ingress_credentials))
}
