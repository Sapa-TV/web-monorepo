use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use crate::orders::game::{GameOrder, GameOrderKind, OrderSource};
use crate::orders::movie::{MovieKind, MovieOrder};
use crate::orders::service::{GameOrderFilter, MovieOrderFilter};
use crate::orders::status::OrderStatus;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct GameOrdersQuery {
    pub status: Option<OrderStatus>,
    pub kind: Option<GameOrderKind>,
    pub source: Option<OrderSource>,
    pub q: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl GameOrdersQuery {
    fn to_filter(&self) -> GameOrderFilter {
        GameOrderFilter {
            status: self.status,
            kind: self.kind,
            source: self.source,
            query: self.q.clone(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct GameOrderResponse {
    pub id: u32,
    pub title: Option<String>,
    pub customer_name: String,
    pub kind: GameOrderKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub completed_at: Option<String>,
    pub comment: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl From<&GameOrder> for GameOrderResponse {
    fn from(order: &GameOrder) -> Self {
        Self {
            id: order.id.get(),
            title: order.title.clone(),
            customer_name: order.customer_name.clone(),
            kind: order.kind,
            source: order.source,
            status: order.status,
            completed_at: order.completed_at.map(|d| d.to_string()),
            comment: order.comment.clone(),
            _sealed: (),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct MovieOrdersQuery {
    pub status: Option<OrderStatus>,
    pub kind: Option<MovieKind>,
    pub source: Option<OrderSource>,
    pub q: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl MovieOrdersQuery {
    fn to_filter(&self) -> MovieOrderFilter {
        MovieOrderFilter {
            status: self.status,
            kind: self.kind,
            source: self.source,
            query: self.q.clone(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct MovieOrderResponse {
    pub id: u32,
    pub title: Option<String>,
    pub customer_name: String,
    pub kind: MovieKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub comment: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl From<&MovieOrder> for MovieOrderResponse {
    fn from(order: &MovieOrder) -> Self {
        Self {
            id: order.id.get(),
            title: order.title.clone(),
            customer_name: order.customer_name.clone(),
            kind: order.kind,
            source: order.source,
            status: order.status,
            comment: order.comment.clone(),
            _sealed: (),
        }
    }
}

#[utoipa::path(get, path = "/orders/games", tag = "orders",
    params(GameOrdersQuery),
    responses((status = 200, description = "Game orders", body = Vec<GameOrderResponse>)))]
pub async fn list_game_orders(
    State(state): State<AppState>,
    Query(query): Query<GameOrdersQuery>,
) -> Result<Json<Vec<GameOrderResponse>>, ApiError> {
    let orders = state.game_order_service.list(query.to_filter()).await?;
    Ok(Json(orders.iter().map(GameOrderResponse::from).collect()))
}

#[utoipa::path(get, path = "/orders/movies", tag = "orders",
    params(MovieOrdersQuery),
    responses((status = 200, description = "Movie orders", body = Vec<MovieOrderResponse>)))]
pub async fn list_movie_orders(
    State(state): State<AppState>,
    Query(query): Query<MovieOrdersQuery>,
) -> Result<Json<Vec<MovieOrderResponse>>, ApiError> {
    let orders = state.movie_order_service.list(query.to_filter()).await?;
    Ok(Json(orders.iter().map(MovieOrderResponse::from).collect()))
}

pub fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_game_orders))
        .routes(routes!(list_movie_orders))
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::orders::game::{GameOrderKind, NewGameOrder, OrderSource};
    use crate::orders::movie::{MovieKind, NewMovieOrder};
    use crate::orders::status::OrderStatus;
    use crate::test_fixtures::{api_path, test_router, test_state};

    async fn get_status(app: &axum::Router, uri: String) -> StatusCode {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        response.status()
    }

    async fn get_json(app: &axum::Router, uri: String) -> (StatusCode, Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        (status, body)
    }

    fn game(
        title: Option<&str>,
        customer: &str,
        kind: GameOrderKind,
        source: OrderSource,
        status: OrderStatus,
    ) -> NewGameOrder {
        NewGameOrder::new(
            title.map(str::to_string),
            customer.to_string(),
            None,
            kind,
            source,
            status,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn game_orders_are_public_and_filterable() {
        let state = test_state().await;
        state
            .game_order_service
            .create(game(
                Some("Noita"),
                "Rikrims",
                GameOrderKind::Stream,
                OrderSource::Roulette,
                OrderStatus::Pending,
            ))
            .await
            .unwrap();
        state
            .game_order_service
            .create(game(
                Some("Starcraft 2"),
                "filneyner",
                GameOrderKind::Stream,
                OrderSource::Donate,
                OrderStatus::Completed,
            ))
            .await
            .unwrap();
        state
            .game_order_service
            .create(game(
                None,
                "Jeker3",
                GameOrderKind::Playthrough,
                OrderSource::Roulette,
                OrderStatus::Pending,
            ))
            .await
            .unwrap();
        let app = test_router(state);

        let (status, body) = get_json(&app, api_path("/orders/games")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_array().unwrap().len(), 3);
        assert_eq!(body[0]["customer_name"], "Rikrims");
        assert_eq!(body[2]["title"], Value::Null);

        let (_, body) = get_json(&app, api_path("/orders/games?status=pending")).await;
        assert_eq!(body.as_array().unwrap().len(), 2);

        let (_, body) = get_json(&app, api_path("/orders/games?kind=playthrough")).await;
        assert_eq!(body.as_array().unwrap().len(), 1);

        let (_, body) = get_json(
            &app,
            api_path("/orders/games?source=roulette&status=pending"),
        )
        .await;
        assert_eq!(body.as_array().unwrap().len(), 2);

        let (_, body) = get_json(&app, api_path("/orders/games?q=star")).await;
        assert_eq!(body.as_array().unwrap().len(), 1);
        assert_eq!(body[0]["customer_name"], "filneyner");

        let (_, body) = get_json(&app, api_path("/orders/games?q=JEKER")).await;
        assert_eq!(body.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn movie_orders_are_public_and_filterable() {
        let state = test_state().await;
        state
            .movie_order_service
            .create(NewMovieOrder::new(
                Some("Большой куш".to_string()),
                "ViyScar".to_string(),
                None,
                MovieKind::Movie,
                OrderSource::Donate,
                OrderStatus::Pending,
                None,
            ))
            .await
            .unwrap();
        state
            .movie_order_service
            .create(NewMovieOrder::new(
                Some("Ре-зеро".to_string()),
                "Jeker3".to_string(),
                None,
                MovieKind::Anime,
                OrderSource::Roulette,
                OrderStatus::Completed,
                None,
            ))
            .await
            .unwrap();
        let app = test_router(state);

        let (status, body) = get_json(&app, api_path("/orders/movies")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_array().unwrap().len(), 2);

        let (_, body) = get_json(&app, api_path("/orders/movies?kind=anime")).await;
        assert_eq!(body.as_array().unwrap().len(), 1);

        let (_, body) = get_json(&app, api_path("/orders/movies?status=completed")).await;
        assert_eq!(body.as_array().unwrap().len(), 1);

        let (_, body) = get_json(&app, api_path("/orders/movies?q=куш")).await;
        assert_eq!(body.as_array().unwrap().len(), 1);
        assert_eq!(body[0]["kind"], "movie");
    }

    #[tokio::test]
    async fn orders_reject_unknown_filter_values() {
        let state = test_state().await;
        let app = test_router(state);

        let status = get_status(&app, api_path("/orders/games?status=bogus")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let status = get_status(&app, api_path("/orders/movies?kind=bogus")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
