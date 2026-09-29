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

#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum IngressPlatform {
    #[default]
    Twitch,
    VkVideoLive,
}

impl IngressPlatform {
    async fn configured(self, state: &AppState) -> bool {
        let res = match self {
            IngressPlatform::Twitch => state.admin_auth.is_ingress_credentials_configured().await,
            IngressPlatform::VkVideoLive => state.vk_admin_auth.is_configured().await,
        };
        match res {
            Ok(configured) => configured,
            Err(e) => {
                tracing::warn!("failed to read {:?} credentials status: {e}", self);
                false
            }
        }
    }

    async fn revoke(self, state: &AppState) -> Result<(), StatusCode> {
        match self {
            IngressPlatform::Twitch => {
                state.admin_auth.revoke_ingress_credentials().await?;
            }
            IngressPlatform::VkVideoLive => {
                state.vk_admin_auth.revoke().await?;
            }
        }
        Ok(())
    }
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
        twitch: IngressPlatform::Twitch.configured(&state).await,
        vk_video_live: IngressPlatform::VkVideoLive.configured(&state).await,
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
    query.platform.revoke(&state).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_ingress_credentials, revoke_ingress_credentials))
}
