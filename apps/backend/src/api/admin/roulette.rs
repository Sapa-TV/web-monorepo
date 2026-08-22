use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use crate::roulette::rarity::{Rarity, RarityId};
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

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertRouletteSlotRequest {
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct SlotIdParam {
    pub id: RouletteSlotId,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RarityResponse {
    pub id: RarityId,
    pub name: String,
    pub display_name: String,
    pub image: String,
    pub color: String,
}

impl From<Rarity> for RarityResponse {
    fn from(rarity: Rarity) -> Self {
        Self {
            id: rarity.id,
            name: rarity.name,
            display_name: rarity.display_name,
            image: rarity.image,
            color: rarity.color,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertRarityRequest {
    pub name: String,
    pub display_name: String,
    pub image: String,
    pub color: String,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct RarityIdParam {
    pub id: RarityId,
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
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::auth::SESSION_COOKIE;
    use crate::roulette::rarity::RarityId;
    use crate::roulette::slot_service::RouletteSlot;
    use crate::roulette::slot_service::RouletteSlotId;
    use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

    const COMMON: RarityId = RarityId::new(1);

    fn slot_body() -> &'static str {
        r#"{"name":"spin","rarity_id":1,"weight":10,"action":"enqueue_roulette"}"#
    }

    #[tokio::test]
    async fn roulette_routes_require_admin_session_cookie() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/roulette/slots"))
                    .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_can_list_roulette_slots() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        state
            .slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "spin",
                COMMON,
                10,
                "enqueue_roulette",
            ))
            .await
            .unwrap();

        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/roulette/slots"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body.as_array().unwrap().len(), 1);
        assert_eq!(body[0]["name"], "spin");
        assert_eq!(body[0]["rarity_id"], 1);
        assert_eq!(body[0]["weight"], 10);
    }

    #[tokio::test]
    async fn admin_can_create_update_and_delete_slot() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/roulette/slots"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .body(Body::from(slot_body()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let slot_id = body["id"].as_u64().unwrap();

        let updated = r#"{"name":"renamed","rarity_id":2,"weight":42,"action":"no_action"}"#;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(api_path(&format!("/admin/roulette/slots/{slot_id}")))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .body(Body::from(updated))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["name"], "renamed");
        assert_eq!(body["weight"], 42);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path(&format!("/admin/roulette/slots/{slot_id}")))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.slot_service.get_slots().is_empty());
    }

    #[tokio::test]
    async fn delete_missing_slot_is_not_found() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path("/admin/roulette/slots/999"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn admin_can_create_update_and_delete_rarity() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/roulette/rarities"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .body(Body::from(
                        r##"{"name":"custom","display_name":"Custom","image":"c.png","color":"#fff"}"##,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let rarity_id = body["id"].as_u64().unwrap();

        let updated =
            r##"{"name":"custom","display_name":"Renamed","image":"c.png","color":"#000"}"##;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(api_path(&format!("/admin/roulette/rarities/{rarity_id}")))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .body(Body::from(updated))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["display_name"], "Renamed");

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path(&format!("/admin/roulette/rarities/{rarity_id}")))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.rarity_service.get_all().is_empty());
    }

    #[tokio::test]
    async fn update_missing_rarity_is_not_found() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(api_path("/admin/roulette/rarities/999"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie)
                    .body(Body::from(
                        r##"{"name":"x","display_name":"X","image":"x.png","color":"#fff"}"##,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
