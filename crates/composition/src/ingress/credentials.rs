use http::StatusCode;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngressPlatform {
    #[default]
    Twitch,
    VkVideoLive,
}

impl IngressPlatform {
    pub async fn credentials_configured(self, state: &AppState) -> bool {
        let result = match self {
            Self::Twitch => state
                .admin_auth
                .is_ingress_credentials_configured()
                .await
                .map_err(|e| e.to_string()),
            Self::VkVideoLive => state
                .vk_admin_auth
                .is_configured()
                .await
                .map_err(|e| e.to_string()),
        };
        result.unwrap_or_else(|e| {
            tracing::warn!("failed to read {self:?} credentials status: {e}");
            false
        })
    }

    pub async fn revoke_credentials(self, state: &AppState) -> Result<(), StatusCode> {
        match self {
            Self::Twitch => state
                .admin_auth
                .revoke_ingress_credentials()
                .await
                .map_err(StatusCode::from),
            Self::VkVideoLive => state.vk_admin_auth.revoke().await.map_err(StatusCode::from),
        }
    }
}
