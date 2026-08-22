use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use crate::roulette::rarity::RarityId;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RouletteSlotResponse {
    pub id: RouletteSlotId,
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
}

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
mod tests {
    use axum::body::Body;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::roulette::rarity::RarityId;
    use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
    use crate::test_fixtures::{test_router, test_state};

    #[tokio::test]
    async fn slots_are_read_only() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/wapi/slots")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer test-key")
                    .body(Body::from(
                        r#"{"name":"x","rarity_id":1,"weight":1,"action":""}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn wak_key_can_list_slots() {
        let state = test_state().await;
        state
            .slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "spin",
                RarityId::new(1),
                10,
                "enqueue_roulette",
            ))
            .await
            .unwrap();
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/wapi/slots")
                    .header(header::AUTHORIZATION, "Bearer test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
