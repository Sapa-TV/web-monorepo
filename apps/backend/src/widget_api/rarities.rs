use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use crate::roulette::dto::RarityResponse;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/rarities",
    tag = "rarities",
    responses(
        (status = 200, description = "List of rarities", body = Vec<RarityResponse>),
    )
)]
pub async fn list_rarities(
    State(state): State<AppState>,
) -> Result<Json<Vec<RarityResponse>>, ApiError> {
    let rarities = state.rarity_service.get_all();
    Ok(Json(
        rarities.into_iter().map(RarityResponse::from).collect(),
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_rarities))
}

#[cfg(test)]
#[path = "rarities.test.rs"]
mod tests;
