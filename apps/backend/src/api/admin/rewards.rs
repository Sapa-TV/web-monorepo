use std::collections::HashSet;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use twitch_api::helix::points::CustomReward;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RewardResponse {
    pub id: String,
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
            title: reward.title,
            cost: reward.cost as u64,
            is_enabled: reward.is_enabled,
            is_paused: reward.is_paused,
            used_in_rules: used_in_rules.contains(&reward.id.to_string()),
            _sealed: (),
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/rewards",
    tag = "admin",
    responses(
        (status = 200, description = "List custom rewards from Twitch", body = Vec<RewardResponse>),
        (status = 400, description = "Twitch is not configured"),
        (status = 401, description = "No valid user token"),
        (status = 502, description = "Twitch API request failed"),
    )
)]
pub async fn list_rewards(
    State(state): State<AppState>,
) -> Result<Json<Vec<RewardResponse>>, StatusCode> {
    let twitch = state.twitch_api.as_ref().ok_or(StatusCode::BAD_REQUEST)?;
    let rewards = twitch.custom_rewards().await.map_err(StatusCode::from)?;
    let used = state
        .rule_service
        .referenced_reward_ids()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rewards
            .into_iter()
            .map(|reward| RewardResponse::build(reward, &used))
            .collect(),
    ))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_rewards))
}

#[cfg(test)]
#[path = "rewards.test.rs"]
mod tests;
