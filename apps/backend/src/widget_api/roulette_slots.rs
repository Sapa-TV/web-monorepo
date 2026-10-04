use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use crate::roulette::dto::RouletteSlotResponse;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/slots",
    tag = "slots",
    responses(
        (status = 200, description = "List all slots", body = Vec<RouletteSlotResponse>)
    )
)]
pub async fn list_slots(
    State(state): State<AppState>,
) -> Result<Json<Vec<RouletteSlotResponse>>, ApiError> {
    let slots = state.slot_service.get_slots();
    Ok(Json(slots.into_iter().map(Into::into).collect()))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_slots))
}

#[cfg(test)]
#[path = "roulette_slots.test.rs"]
mod tests;
