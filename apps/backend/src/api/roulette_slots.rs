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
    let updated = state
        .slot_repo
        .update(slot)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "slot not found"))?;
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
    let deleted =     state.slot_repo.delete(params.id).await?;
    if !deleted {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "slot not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use serde_json::Value;
    use tower::ServiceExt;

    use std::sync::Arc;

    use crate::api::router;
    use crate::db::inmemory_platform::InMemoryPlatformRepository;
    use crate::db::inmemory_rarity::InMemoryRarityRepository;
    use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
    use crate::db::inmemory_user::InMemoryUserRepository;
    use crate::random::StandartRandomProvider;
    use crate::roulette::repository::RouletteSlotRepository;
    use crate::state::AppState;

    fn test_state() -> AppState {
        AppState {
            slot_repo: Arc::new(InMemoryRouletteSlotRepository::new()),
            rarity_repo: Arc::new(InMemoryRarityRepository::new()),
            user_repo: Arc::new(InMemoryUserRepository::new()),
            platform_repo: Arc::new(InMemoryPlatformRepository::new_seeded()),
            random: StandartRandomProvider,
        }
    }

    #[tokio::test]
    async fn list_slots_empty() {
        let state = test_state();
        let app = Router::new().merge(router()).with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/slots")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn create_slot_201() {
        let state = test_state();
        let app = Router::new().merge(router()).with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/slots")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"name":"test","rarity_id":1,"weight":10,"action":"act"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["name"], "test");
        assert_eq!(body["id"], 1);
    }

    #[tokio::test]
    async fn update_slot_200() {
        let state = test_state();
        let app = Router::new().merge(router()).with_state(state.clone());

        let saved = state
            .slot_repo
            .save(crate::roulette::slot_service::RouletteSlot::new(
                crate::roulette::slot_service::RouletteSlotId::new(0),
                "original",
                crate::roulette::rarity::RarityId::new(1),
                10,
                "act",
            ))
            .await
            .unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(&format!("/api/slots/{}", saved.id.value()))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"name":"updated","rarity_id":1,"weight":99,"action":"new_act"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["name"], "updated");
        assert_eq!(body["weight"], 99);
    }

    #[tokio::test]
    async fn update_slot_404() {
        let state = test_state();
        let app = Router::new().merge(router()).with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/api/slots/999")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"name":"nonexistent","rarity_id":1,"weight":1,"action":"act"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn delete_slot_204() {
        let state = test_state();
        let app = Router::new().merge(router()).with_state(state.clone());

        let saved = state
            .slot_repo
            .save(crate::roulette::slot_service::RouletteSlot::new(
                crate::roulette::slot_service::RouletteSlotId::new(0),
                "to_delete",
                crate::roulette::rarity::RarityId::new(1),
                10,
                "act",
            ))
            .await
            .unwrap();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(&format!("/api/slots/{}", saved.id.value()))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 204);
    }

    #[tokio::test]
    async fn delete_slot_404() {
        let state = test_state();
        let app = Router::new().merge(router()).with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri("/api/slots/999")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }
}