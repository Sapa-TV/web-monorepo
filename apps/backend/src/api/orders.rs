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
#[path = "orders.test.rs"]
mod tests;
