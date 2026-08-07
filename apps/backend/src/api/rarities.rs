use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use utoipa::OpenApi;
use utoipa::ToSchema;

use crate::error::api::ApiError;
use crate::roulette::rarity::{Rarity, RarityId};
use crate::state::AppState;

#[derive(Debug, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct RarityResponse {
    pub id: RarityId,
    pub name: String,
    pub display_name: String,
    pub image: String,
    pub color: String,
}

impl From<Rarity> for RarityResponse {
    fn from(r: Rarity) -> Self {
        Self {
            id: r.id,
            name: r.name,
            display_name: r.display_name,
            image: r.image,
            color: r.color,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct CreateRarityRequest {
    pub name: String,
    pub display_name: String,
    pub image: String,
    pub color: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpdateRarityRequest {
    pub name: String,
    pub display_name: String,
    pub image: String,
    pub color: String,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[non_exhaustive]
pub struct RarityIdParam {
    pub id: RarityId,
}

#[utoipa::path(
    get,
    path = "/api/rarities",
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

#[utoipa::path(
    post,
    path = "/api/rarities",
    tag = "rarities",
    request_body = CreateRarityRequest,
    responses(
        (status = 201, description = "Rarity created", body = RarityResponse),
    )
)]
pub async fn create_rarity(
    State(state): State<AppState>,
    Json(body): Json<CreateRarityRequest>,
) -> Result<(StatusCode, Json<RarityResponse>), ApiError> {
    let rarity = Rarity::new(
        RarityId::new(0),
        &body.name,
        &body.display_name,
        &body.image,
        &body.color,
    );
    let saved = state.rarity_service.save(rarity).await?;
    Ok((StatusCode::CREATED, Json(RarityResponse::from(saved))))
}

#[utoipa::path(
    put,
    path = "/api/rarities/{id}",
    tag = "rarities",
    params(RarityIdParam),
    request_body = UpdateRarityRequest,
    responses(
        (status = 200, description = "Rarity updated", body = RarityResponse),
        (status = 404, description = "Rarity not found"),
    )
)]
pub async fn update_rarity(
    State(state): State<AppState>,
    Path(params): Path<RarityIdParam>,
    Json(body): Json<UpdateRarityRequest>,
) -> Result<Json<RarityResponse>, ApiError> {
    let rarity = Rarity::new(
        params.id,
        &body.name,
        &body.display_name,
        &body.image,
        &body.color,
    );
    let updated = state
        .rarity_service
        .update(rarity)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "rarity not found"))?;
    Ok(Json(RarityResponse::from(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/rarities/{id}",
    tag = "rarities",
    params(RarityIdParam),
    responses(
        (status = 204, description = "Rarity deleted"),
        (status = 404, description = "Rarity not found"),
    )
)]
pub async fn delete_rarity(
    State(state): State<AppState>,
    Path(params): Path<RarityIdParam>,
) -> Result<StatusCode, ApiError> {
    let deleted = state.rarity_service.delete(params.id).await?;
    if !deleted {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "rarity not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(OpenApi)]
#[openapi(
    paths(list_rarities, create_rarity, update_rarity, delete_rarity),
    components(schemas(RarityResponse, CreateRarityRequest, UpdateRarityRequest,))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct RaritiesApiDoc;

pub fn protected_router() -> axum::Router<AppState> {
    use axum::routing::{delete, get, post, put};
    axum::Router::new()
        .route("/api/rarities", get(list_rarities))
        .route("/api/rarities", post(create_rarity))
        .route("/api/rarities/{id}", put(update_rarity))
        .route("/api/rarities/{id}", delete(delete_rarity))
}
