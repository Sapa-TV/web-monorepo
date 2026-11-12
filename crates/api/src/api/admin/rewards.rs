use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use composition::rewards::{RewardListing, list as list_platform_rewards};
use composition::state::AppState;
use model::platform::PlatformId;

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

impl From<RewardListing> for RewardResponse {
    fn from(listing: RewardListing) -> Self {
        Self {
            id: listing.id,
            platform: listing.platform,
            title: listing.title,
            cost: listing.cost,
            is_enabled: listing.is_enabled,
            is_paused: listing.is_paused,
            used_in_rules: listing.used_in_rules,
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
    let rewards = list_platform_rewards(&state).await?;
    Ok(Json(
        rewards.into_iter().map(RewardResponse::from).collect(),
    ))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_rewards))
}

#[cfg(test)]
#[path = "rewards.test.rs"]
mod tests;
