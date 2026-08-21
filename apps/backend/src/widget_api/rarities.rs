use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::OpenApi;
use utoipa::ToSchema;

use crate::error::api::ApiError;
use crate::roulette::rarity::{Rarity, RarityId};
use crate::state::AppState;

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

#[derive(OpenApi)]
#[openapi(paths(list_rarities), components(schemas(RarityResponse,)))]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct RaritiesApiDoc;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/rarities", get(list_rarities))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::roulette::rarity::{Rarity, RarityId};
    use crate::test_fixtures::{test_router, test_state};

    #[tokio::test]
    async fn rarities_are_read_only() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/wapi/rarities")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer test-key")
                    .body(Body::from(
                        r##"{"name":"x","display_name":"X","image":"x.png","color":"#fff"}"##,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn wak_key_can_list_rarities() {
        let state = test_state().await;
        state
            .rarity_service
            .save(Rarity::new(
                RarityId::new(0),
                "custom",
                "Custom",
                "c.png",
                "#fff",
            ))
            .await
            .unwrap();
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/wapi/rarities")
                    .header(header::AUTHORIZATION, "Bearer test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
