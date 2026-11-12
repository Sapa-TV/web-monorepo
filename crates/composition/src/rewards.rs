use std::collections::HashSet;

use http::StatusCode;
use twitch_api::helix::points::CustomReward;
use vk_video_live::api;

use crate::state::AppState;
use model::platform::PlatformId;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RewardListing {
    pub id: String,
    pub platform: PlatformId,
    pub title: String,
    pub cost: u64,
    pub is_enabled: bool,
    pub is_paused: bool,
    pub used_in_rules: bool,
}

impl RewardListing {
    fn from_twitch(reward: CustomReward, used: &HashSet<String>) -> Self {
        let id = reward.id.to_string();
        Self {
            used_in_rules: used.contains(&id),
            id,
            platform: PlatformId::TWITCH,
            title: reward.title,
            cost: reward.cost as u64,
            is_enabled: reward.is_enabled,
            is_paused: reward.is_paused,
        }
    }

    fn from_vk(reward: api::RewardInfo, used: &HashSet<String>) -> Self {
        Self {
            used_in_rules: used.contains(&reward.id),
            id: reward.id,
            platform: PlatformId::VK_VIDEO_LIVE,
            title: reward.name,
            cost: reward.price,
            is_enabled: !reward.is_disabled,
            is_paused: false,
        }
    }
}

pub async fn list(state: &AppState) -> Result<Vec<RewardListing>, StatusCode> {
    let used = state
        .rule_service
        .referenced_reward_ids()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut listings = twitch_rewards(state, &used).await?;
    listings.extend(vk_rewards(state, &used).await?);
    Ok(listings)
}

async fn twitch_rewards(
    state: &AppState,
    used: &HashSet<String>,
) -> Result<Vec<RewardListing>, StatusCode> {
    let Some(twitch) = state.twitch_api.as_ref() else {
        return Ok(Vec::new());
    };
    let rewards = twitch.custom_rewards().await.map_err(|e| e.status_code())?;
    Ok(rewards
        .into_iter()
        .map(|reward| RewardListing::from_twitch(reward, used))
        .collect())
}

async fn vk_rewards(
    state: &AppState,
    used: &HashSet<String>,
) -> Result<Vec<RewardListing>, StatusCode> {
    let Some(vk) = state.vk_api.as_ref() else {
        return Ok(Vec::new());
    };
    let Ok(bearer) = vk.bearer().await else {
        tracing::warn!("vk rewards skipped: credentials not configured");
        return Ok(Vec::new());
    };
    let channel_url = vk
        .channel_url()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rewards = api::rewards(vk.transport(), &bearer, &channel_url)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(rewards
        .into_iter()
        .map(|reward| RewardListing::from_vk(reward, used))
        .collect())
}
