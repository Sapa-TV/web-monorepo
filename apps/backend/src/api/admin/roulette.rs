use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use crate::roulette::dto::{RarityResponse, RouletteSlotResponse};
use crate::roulette::rarity::Rarity;
use crate::roulette::rarity::RarityId;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertRouletteSlotRequest {
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct SlotIdParam {
    pub id: RouletteSlotId,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertRarityRequest {
    pub name: String,
    pub display_name: String,
    pub image: String,
    pub color: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct RarityIdParam {
    pub id: RarityId,
    #[serde(skip)]
    _sealed: (),
}

#[utoipa::path(
    get,
    path = "/admin/roulette/slots",
    tag = "admin",
    responses(
        (status = 200, description = "List all roulette slots", body = Vec<RouletteSlotResponse>),
    )
)]
pub async fn list_slots(State(state): State<AppState>) -> Json<Vec<RouletteSlotResponse>> {
    Json(
        state
            .slot_service
            .get_slots()
            .into_iter()
            .map(RouletteSlotResponse::from)
            .collect(),
    )
}

#[utoipa::path(
    post,
    path = "/admin/roulette/slots",
    tag = "admin",
    request_body = UpsertRouletteSlotRequest,
    responses(
        (status = 201, description = "Roulette slot created", body = RouletteSlotResponse),
    )
)]
pub async fn create_slot(
    State(state): State<AppState>,
    Json(body): Json<UpsertRouletteSlotRequest>,
) -> Result<(StatusCode, Json<RouletteSlotResponse>), ApiError> {
    let slot = state
        .slot_service
        .add_slot(RouletteSlot::new(
            RouletteSlotId::new(0),
            body.name,
            body.rarity_id,
            body.weight,
            body.action,
        ))
        .await?;
    Ok((StatusCode::CREATED, Json(RouletteSlotResponse::from(slot))))
}

#[utoipa::path(
    put,
    path = "/admin/roulette/slots/{id}",
    tag = "admin",
    params(SlotIdParam),
    request_body = UpsertRouletteSlotRequest,
    responses(
        (status = 200, description = "Roulette slot updated", body = RouletteSlotResponse),
        (status = 404, description = "Roulette slot not found"),
    )
)]
pub async fn update_slot(
    State(state): State<AppState>,
    Path(param): Path<SlotIdParam>,
    Json(body): Json<UpsertRouletteSlotRequest>,
) -> Result<Json<RouletteSlotResponse>, ApiError> {
    let updated = state
        .slot_service
        .edit_slot(RouletteSlot::new(
            param.id,
            body.name,
            body.rarity_id,
            body.weight,
            body.action,
        ))
        .await?;
    let updated =
        updated.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Roulette slot not found"))?;
    Ok(Json(RouletteSlotResponse::from(updated)))
}

#[utoipa::path(
    delete,
    path = "/admin/roulette/slots/{id}",
    tag = "admin",
    params(SlotIdParam),
    responses(
        (status = 204, description = "Roulette slot removed"),
        (status = 404, description = "Roulette slot not found"),
    )
)]
pub async fn delete_slot(
    State(state): State<AppState>,
    Path(param): Path<SlotIdParam>,
) -> Result<StatusCode, ApiError> {
    let deleted = state.slot_service.delete_slot(param.id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "Roulette slot not found",
        ))
    }
}

#[utoipa::path(
    get,
    path = "/admin/roulette/rarities",
    tag = "admin",
    responses(
        (status = 200, description = "List all rarities", body = Vec<RarityResponse>),
    )
)]
pub async fn list_rarities(State(state): State<AppState>) -> Json<Vec<RarityResponse>> {
    Json(
        state
            .rarity_service
            .get_all()
            .into_iter()
            .map(RarityResponse::from)
            .collect(),
    )
}

#[utoipa::path(
    post,
    path = "/admin/roulette/rarities",
    tag = "admin",
    request_body = UpsertRarityRequest,
    responses(
        (status = 201, description = "Rarity created", body = RarityResponse),
    )
)]
pub async fn create_rarity(
    State(state): State<AppState>,
    Json(body): Json<UpsertRarityRequest>,
) -> Result<(StatusCode, Json<RarityResponse>), ApiError> {
    let rarity = state
        .rarity_service
        .save(Rarity::new(
            RarityId::new(0),
            body.name,
            body.display_name,
            body.image,
            body.color,
        ))
        .await?;
    Ok((StatusCode::CREATED, Json(RarityResponse::from(rarity))))
}

#[utoipa::path(
    put,
    path = "/admin/roulette/rarities/{id}",
    tag = "admin",
    params(RarityIdParam),
    request_body = UpsertRarityRequest,
    responses(
        (status = 200, description = "Rarity updated", body = RarityResponse),
        (status = 404, description = "Rarity not found"),
    )
)]
pub async fn update_rarity(
    State(state): State<AppState>,
    Path(param): Path<RarityIdParam>,
    Json(body): Json<UpsertRarityRequest>,
) -> Result<Json<RarityResponse>, ApiError> {
    let updated = state
        .rarity_service
        .update(Rarity::new(
            param.id,
            body.name,
            body.display_name,
            body.image,
            body.color,
        ))
        .await?;
    let updated =
        updated.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Rarity not found"))?;
    Ok(Json(RarityResponse::from(updated)))
}

#[utoipa::path(
    delete,
    path = "/admin/roulette/rarities/{id}",
    tag = "admin",
    params(RarityIdParam),
    responses(
        (status = 204, description = "Rarity removed"),
        (status = 404, description = "Rarity not found"),
    )
)]
pub async fn delete_rarity(
    State(state): State<AppState>,
    Path(param): Path<RarityIdParam>,
) -> Result<StatusCode, ApiError> {
    let deleted = state.rarity_service.delete(param.id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new(StatusCode::NOT_FOUND, "Rarity not found"))
    }
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_slots, create_slot))
        .routes(routes!(update_slot, delete_slot))
        .routes(routes!(list_rarities, create_rarity))
        .routes(routes!(update_rarity, delete_rarity))
}

#[cfg(test)]
#[path = "roulette.test.rs"]
mod tests;
