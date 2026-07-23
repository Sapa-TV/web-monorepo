use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;
use utoipa::{IntoParams, ToSchema};

use crate::error::api::ApiError;
use crate::roulette::rarity::RarityId;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct RouletteSlotResponse {
    pub id: RouletteSlotId,
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRouletteSlotRequest {
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRouletteSlotRequest {
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SlotIdParam {
    pub id: RouletteSlotId,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_slots, create_slot, update_slot, delete_slot),
    components(schemas(
        RouletteSlotResponse,
        CreateRouletteSlotRequest,
        UpdateRouletteSlotRequest,
    ))
)]
pub(crate) struct SlotsApiDoc;

impl From<RouletteSlot> for RouletteSlotResponse {
    fn from(slot: RouletteSlot) -> Self {
        Self {
            id: slot.id,
            name: slot.name,
            rarity_id: slot.rarity_id,
            weight: slot.weight,
            action: slot.action,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/slots",
    tag = "slots",
    responses(
        (status = 200, description = "List all slots", body = Vec<RouletteSlotResponse>)
    )
)]
pub async fn list_slots(
    State(state): State<AppState>,
) -> Result<Json<Vec<RouletteSlotResponse>>, ApiError> {
    let slots = state.slot_repo.load_all().await?;
    Ok(Json(slots.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/slots",
    tag = "slots",
    request_body = CreateRouletteSlotRequest,
    responses(
        (status = 201, description = "Slot created", body = RouletteSlotResponse),
        (status = 400, description = "Invalid input")
    )
)]
pub async fn create_slot(
    State(state): State<AppState>,
    Json(body): Json<CreateRouletteSlotRequest>,
) -> Result<(StatusCode, Json<RouletteSlotResponse>), ApiError> {
    let slot = RouletteSlot::new(
        RouletteSlotId::new(0),
        &body.name,
        body.rarity_id,
        body.weight,
        &body.action,
    );
    let saved = state.slot_repo.save(slot).await?;
    Ok((StatusCode::CREATED, Json(saved.into())))
}

#[utoipa::path(
    put,
    path = "/api/slots/{id}",
    tag = "slots",
    params(SlotIdParam),
    request_body = UpdateRouletteSlotRequest,
    responses(
        (status = 200, description = "Slot updated", body = RouletteSlotResponse),
        (status = 404, description = "Slot not found")
    )
)]
pub async fn update_slot(
    State(state): State<AppState>,
    Path(params): Path<SlotIdParam>,
    Json(body): Json<UpdateRouletteSlotRequest>,
) -> Result<Json<RouletteSlotResponse>, ApiError> {
    let slot = RouletteSlot::new(
        params.id,
        &body.name,
        body.rarity_id,
        body.weight,
        &body.action,
    );
    let updated = state.slot_repo.update(slot).await?;
    Ok(Json(updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/slots/{id}",
    tag = "slots",
    params(SlotIdParam),
    responses(
        (status = 204, description = "Slot deleted"),
        (status = 404, description = "Slot not found")
    )
)]
pub async fn delete_slot(
    State(state): State<AppState>,
    Path(params): Path<SlotIdParam>,
) -> Result<StatusCode, ApiError> {
    state.slot_repo.delete(params.id).await?;
    Ok(StatusCode::NO_CONTENT)
}