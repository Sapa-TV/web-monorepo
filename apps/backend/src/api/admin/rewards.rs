use std::collections::HashSet;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use twitch_api::helix::points::CustomReward;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use vk_video_live::api;

use crate::platform::PlatformId;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RewardResponse {
    pub id: String,
    pub platform: PlatformId,
    pub title: String,
    pub cost: u64,
    pub is_enabled: bool,
    pub is_paused: bool,
    pub used_in_rules: bool,
    #[serde(skip)]
    _sealed: (),
}

impl RewardResponse {
    fn build(reward: CustomReward, used_in_rules: &HashSet<String>) -> Self {
        Self {
            id: reward.id.to_string(),
            platform: PlatformId::TWITCH,
            title: reward.title,
            cost: reward.cost as u64,
            is_enabled: reward.is_enabled,
            is_paused: reward.is_paused,
            used_in_rules: used_in_rules.contains(&reward.id.to_string()),
            _sealed: (),
        }
    }

    fn from_vk(reward: api::RewardInfo, used_in_rules: &HashSet<String>) -> Self {
        Self {
            id: reward.id.clone(),
            platform: PlatformId::VK_VIDEO_LIVE,
            title: reward.name,
            cost: reward.price,
            is_enabled: !reward.is_disabled,
            is_paused: false,
            used_in_rules: used_in_rules.contains(&reward.id),
            _sealed: (),
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/rewards",
    tag = "admin",
    responses(
        (status = 200, description = "List custom rewards from configured platforms", body = Vec<RewardResponse>),
        (status = 502, description = "Platform API request failed"),
    )
)]
pub async fn list_rewards(
    State(state): State<AppState>,
) -> Result<Json<Vec<RewardResponse>>, StatusCode> {
    let used = state
        .rule_service
        .referenced_reward_ids()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::new();

    if let Some(twitch) = state.twitch_api.as_ref() {
        let rewards = twitch.custom_rewards().await.map_err(StatusCode::from)?;
        out.extend(
            rewards
                .into_iter()
                .map(|reward| RewardResponse::build(reward, &used)),
        );
    }

    if let Some(vk) = state.vk_api.as_ref() {
        match vk.bearer().await {
            Ok(bearer) => {
                let channel_url = vk
                    .channel_url()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let rewards = api::rewards(vk.transport(), &bearer, &channel_url)
                    .await
                    .map_err(|_| StatusCode::BAD_GATEWAY)?;
                out.extend(
                    rewards
                        .into_iter()
                        .map(|reward| RewardResponse::from_vk(reward, &used)),
                );
            }
            Err(_) => tracing::warn!("vk rewards skipped: credentials not configured"),
        }
    }

    Ok(Json(out))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_rewards))
}

#[cfg(test)]
#[path = "rewards.test.rs"]
mod tests;
