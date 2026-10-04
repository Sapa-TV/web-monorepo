use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use crate::orders::game::{GameOrder, GameOrderId, GameOrderKind, NewGameOrder, OrderSource};
use crate::orders::movie::{MovieKind, MovieOrder, MovieOrderId, NewMovieOrder};
use crate::orders::status::OrderStatus;
use crate::orders::vip::{
    NewVipRecord, VipKind, VipRecord, VipRecordId, VipRecordUpdate, VipStatus,
};
use crate::sheets::service::{ImportCount, ImportReport, SyncReport};
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct OrderIdParam {
    pub id: u32,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertGameOrderRequest {
    pub title: Option<String>,
    pub customer_name: String,
    pub kind: GameOrderKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub completed_at: Option<NaiveDate>,
    pub comment: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl UpsertGameOrderRequest {
    fn into_new(self) -> NewGameOrder {
        NewGameOrder::new(
            self.title,
            self.customer_name,
            None,
            self.kind,
            self.source,
            self.status,
            self.completed_at,
            self.comment,
        )
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct AdminGameOrderResponse {
    pub id: u32,
    pub title: Option<String>,
    pub customer_name: String,
    pub user_id: Option<u32>,
    pub kind: GameOrderKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub completed_at: Option<NaiveDate>,
    pub comment: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<GameOrder> for AdminGameOrderResponse {
    fn from(order: GameOrder) -> Self {
        Self {
            id: order.id.get(),
            title: order.title,
            customer_name: order.customer_name,
            user_id: order.user_id.map(|id| id.get()),
            kind: order.kind,
            source: order.source,
            status: order.status,
            completed_at: order.completed_at,
            comment: order.comment,
            created_at: order.created_at.to_rfc3339(),
            updated_at: order.updated_at.to_rfc3339(),
            _sealed: (),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertMovieOrderRequest {
    pub title: Option<String>,
    pub customer_name: String,
    pub kind: MovieKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub comment: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl UpsertMovieOrderRequest {
    fn into_new(self) -> NewMovieOrder {
        NewMovieOrder::new(
            self.title,
            self.customer_name,
            None,
            self.kind,
            self.source,
            self.status,
            self.comment,
        )
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct AdminMovieOrderResponse {
    pub id: u32,
    pub title: Option<String>,
    pub customer_name: String,
    pub user_id: Option<u32>,
    pub kind: MovieKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub comment: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<MovieOrder> for AdminMovieOrderResponse {
    fn from(order: MovieOrder) -> Self {
        Self {
            id: order.id.get(),
            title: order.title,
            customer_name: order.customer_name,
            user_id: order.user_id.map(|id| id.get()),
            kind: order.kind,
            source: order.source,
            status: order.status,
            comment: order.comment,
            created_at: order.created_at.to_rfc3339(),
            updated_at: order.updated_at.to_rfc3339(),
            _sealed: (),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct CreateVipRecordRequest {
    pub customer_name: String,
    pub kind: VipKind,
    pub roulette_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub note: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl CreateVipRecordRequest {
    fn into_new(self) -> NewVipRecord {
        NewVipRecord::new(
            self.customer_name,
            None,
            self.kind,
            self.roulette_date,
            self.end_date,
            self.note,
        )
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpdateVipRecordRequest {
    pub customer_name: String,
    pub kind: VipKind,
    pub roulette_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: VipStatus,
    pub note: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl UpdateVipRecordRequest {
    fn into_update(self) -> VipRecordUpdate {
        VipRecordUpdate::new(
            self.customer_name,
            self.kind,
            self.roulette_date,
            self.end_date,
            self.status,
            self.note,
        )
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct VipRecordResponse {
    pub id: u32,
    pub customer_name: String,
    pub user_id: Option<u32>,
    pub kind: VipKind,
    pub roulette_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: VipStatus,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<VipRecord> for VipRecordResponse {
    fn from(record: VipRecord) -> Self {
        Self {
            id: record.id.get(),
            customer_name: record.customer_name,
            user_id: record.user_id.map(|id| id.get()),
            kind: record.kind,
            roulette_date: record.roulette_date,
            end_date: record.end_date,
            status: record.status,
            note: record.note,
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
            _sealed: (),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct VipRemindersQuery {
    pub days: Option<u64>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct VipRemindersResponse {
    pub expiring: Vec<VipRecordResponse>,
    pub awaiting_return: Vec<VipRecordResponse>,
    #[serde(skip)]
    _sealed: (),
}

#[utoipa::path(post, path = "/admin/orders/games", tag = "admin",
    request_body = UpsertGameOrderRequest,
    responses((status = 201, description = "Game order created", body = AdminGameOrderResponse)))]
pub async fn create_game_order(
    State(state): State<AppState>,
    Json(body): Json<UpsertGameOrderRequest>,
) -> Result<(StatusCode, Json<AdminGameOrderResponse>), ApiError> {
    let order = state.game_order_service.create(body.into_new()).await?;
    state.sheets.mark_dirty();
    Ok((
        StatusCode::CREATED,
        Json(AdminGameOrderResponse::from(order)),
    ))
}

#[utoipa::path(put, path = "/admin/orders/games/{id}", tag = "admin",
    params(OrderIdParam),
    request_body = UpsertGameOrderRequest,
    responses((status = 200, description = "Game order updated", body = AdminGameOrderResponse),
        (status = 404, description = "Game order not found")))]
pub async fn update_game_order(
    State(state): State<AppState>,
    Path(params): Path<OrderIdParam>,
    Json(body): Json<UpsertGameOrderRequest>,
) -> Result<Json<AdminGameOrderResponse>, ApiError> {
    let order = state
        .game_order_service
        .replace(GameOrderId::new(params.id), body.into_new())
        .await?;
    state.sheets.mark_dirty();
    Ok(Json(AdminGameOrderResponse::from(order)))
}

#[utoipa::path(delete, path = "/admin/orders/games/{id}", tag = "admin",
    params(OrderIdParam),
    responses((status = 204, description = "Game order deleted"),
        (status = 404, description = "Game order not found")))]
pub async fn delete_game_order(
    State(state): State<AppState>,
    Path(params): Path<OrderIdParam>,
) -> Result<StatusCode, ApiError> {
    state
        .game_order_service
        .delete(GameOrderId::new(params.id))
        .await?;
    state.sheets.mark_dirty();
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/admin/orders/movies", tag = "admin",
    request_body = UpsertMovieOrderRequest,
    responses((status = 201, description = "Movie order created", body = AdminMovieOrderResponse)))]
pub async fn create_movie_order(
    State(state): State<AppState>,
    Json(body): Json<UpsertMovieOrderRequest>,
) -> Result<(StatusCode, Json<AdminMovieOrderResponse>), ApiError> {
    let order = state.movie_order_service.create(body.into_new()).await?;
    state.sheets.mark_dirty();
    Ok((
        StatusCode::CREATED,
        Json(AdminMovieOrderResponse::from(order)),
    ))
}

#[utoipa::path(put, path = "/admin/orders/movies/{id}", tag = "admin",
    params(OrderIdParam),
    request_body = UpsertMovieOrderRequest,
    responses((status = 200, description = "Movie order updated", body = AdminMovieOrderResponse),
        (status = 404, description = "Movie order not found")))]
pub async fn update_movie_order(
    State(state): State<AppState>,
    Path(params): Path<OrderIdParam>,
    Json(body): Json<UpsertMovieOrderRequest>,
) -> Result<Json<AdminMovieOrderResponse>, ApiError> {
    let order = state
        .movie_order_service
        .replace(MovieOrderId::new(params.id), body.into_new())
        .await?;
    state.sheets.mark_dirty();
    Ok(Json(AdminMovieOrderResponse::from(order)))
}

#[utoipa::path(delete, path = "/admin/orders/movies/{id}", tag = "admin",
    params(OrderIdParam),
    responses((status = 204, description = "Movie order deleted"),
        (status = 404, description = "Movie order not found")))]
pub async fn delete_movie_order(
    State(state): State<AppState>,
    Path(params): Path<OrderIdParam>,
) -> Result<StatusCode, ApiError> {
    state
        .movie_order_service
        .delete(MovieOrderId::new(params.id))
        .await?;
    state.sheets.mark_dirty();
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/admin/orders/vip", tag = "admin",
    responses((status = 200, description = "All VIP records", body = Vec<VipRecordResponse>)))]
pub async fn list_vip_records(
    State(state): State<AppState>,
) -> Result<Json<Vec<VipRecordResponse>>, ApiError> {
    let records = state.vip_service.list(Default::default()).await?;
    Ok(Json(
        records.into_iter().map(VipRecordResponse::from).collect(),
    ))
}

#[utoipa::path(get, path = "/admin/orders/vip/reminders", tag = "admin",
    params(VipRemindersQuery),
    responses((status = 200, description = "VIP reminders", body = VipRemindersResponse)))]
pub async fn vip_reminders(
    State(state): State<AppState>,
    Query(query): Query<VipRemindersQuery>,
) -> Result<Json<VipRemindersResponse>, ApiError> {
    let today = Utc::now().date_naive();
    let days = query.days.unwrap_or(3);
    let expiring = state.vip_service.expiring_within(today, days).await?;
    let awaiting_return = state.vip_service.awaiting_return(today).await?;
    Ok(Json(VipRemindersResponse {
        expiring: expiring.into_iter().map(VipRecordResponse::from).collect(),
        awaiting_return: awaiting_return
            .into_iter()
            .map(VipRecordResponse::from)
            .collect(),
        _sealed: (),
    }))
}

#[utoipa::path(post, path = "/admin/orders/vip", tag = "admin",
    request_body = CreateVipRecordRequest,
    responses((status = 201, description = "VIP record created", body = VipRecordResponse)))]
pub async fn create_vip_record(
    State(state): State<AppState>,
    Json(body): Json<CreateVipRecordRequest>,
) -> Result<(StatusCode, Json<VipRecordResponse>), ApiError> {
    let record = state.vip_service.create(body.into_new()).await?;
    state.sheets.mark_dirty();
    Ok((StatusCode::CREATED, Json(VipRecordResponse::from(record))))
}

#[utoipa::path(put, path = "/admin/orders/vip/{id}", tag = "admin",
    params(OrderIdParam),
    request_body = UpdateVipRecordRequest,
    responses((status = 200, description = "VIP record updated", body = VipRecordResponse),
        (status = 404, description = "VIP record not found")))]
pub async fn update_vip_record(
    State(state): State<AppState>,
    Path(params): Path<OrderIdParam>,
    Json(body): Json<UpdateVipRecordRequest>,
) -> Result<Json<VipRecordResponse>, ApiError> {
    let record = state
        .vip_service
        .replace(VipRecordId::new(params.id), body.into_update())
        .await?;
    state.sheets.mark_dirty();
    Ok(Json(VipRecordResponse::from(record)))
}

#[utoipa::path(delete, path = "/admin/orders/vip/{id}", tag = "admin",
    params(OrderIdParam),
    responses((status = 204, description = "VIP record deleted"),
        (status = 404, description = "VIP record not found")))]
pub async fn delete_vip_record(
    State(state): State<AppState>,
    Path(params): Path<OrderIdParam>,
) -> Result<StatusCode, ApiError> {
    state
        .vip_service
        .delete(VipRecordId::new(params.id))
        .await?;
    state.sheets.mark_dirty();
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct SheetsStatusResponse {
    pub configured: bool,
    pub spreadsheet_id: String,
    pub last_synced_at: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct ImportRequest {
    pub spreadsheet_url: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ImportCountResponse {
    pub imported: u32,
    pub skipped: u32,
    #[serde(skip)]
    _sealed: (),
}

impl From<ImportCount> for ImportCountResponse {
    fn from(count: ImportCount) -> Self {
        Self {
            imported: count.imported,
            skipped: count.skipped,
            _sealed: (),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ImportReportResponse {
    pub spreadsheet_id: String,
    pub games: ImportCountResponse,
    pub movies: ImportCountResponse,
    pub vip: ImportCountResponse,
    #[serde(skip)]
    _sealed: (),
}

impl From<ImportReport> for ImportReportResponse {
    fn from(report: ImportReport) -> Self {
        Self {
            spreadsheet_id: report.spreadsheet_id,
            games: ImportCountResponse::from(report.games),
            movies: ImportCountResponse::from(report.movies),
            vip: ImportCountResponse::from(report.vip),
            _sealed: (),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct SyncReportResponse {
    pub games: u32,
    pub movies: u32,
    pub vip: u32,
    pub synced_at: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<SyncReport> for SyncReportResponse {
    fn from(report: SyncReport) -> Self {
        Self {
            games: report.games,
            movies: report.movies,
            vip: report.vip,
            synced_at: report.synced_at.to_rfc3339(),
            _sealed: (),
        }
    }
}

#[utoipa::path(get, path = "/admin/orders/sheets", tag = "admin",
    responses((status = 200, description = "Sheets integration status", body = SheetsStatusResponse)))]
pub async fn sheets_status(State(state): State<AppState>) -> Json<SheetsStatusResponse> {
    Json(SheetsStatusResponse {
        configured: state.sheets.configured(),
        spreadsheet_id: state.sheets.spreadsheet_id(),
        last_synced_at: state.sheets.last_synced_at().map(|t| t.to_rfc3339()),
        _sealed: (),
    })
}

#[utoipa::path(post, path = "/admin/orders/sync", tag = "admin",
    responses((status = 200, description = "Sync report", body = SyncReportResponse),
        (status = 400, description = "Not configured or spreadsheet id missing"),
        (status = 502, description = "Google API error")))]
pub async fn sync_orders(
    State(state): State<AppState>,
) -> Result<Json<SyncReportResponse>, ApiError> {
    let report = state.sheets.sync().await?;
    Ok(Json(SyncReportResponse::from(report)))
}

#[utoipa::path(post, path = "/admin/orders/import", tag = "admin",
    request_body = ImportRequest,
    responses((status = 200, description = "Import report", body = ImportReportResponse),
        (status = 400, description = "Not configured or invalid url"),
        (status = 502, description = "Google API error")))]
pub async fn import_orders(
    State(state): State<AppState>,
    Json(body): Json<ImportRequest>,
) -> Result<Json<ImportReportResponse>, ApiError> {
    let report = state.sheets.import(&body.spreadsheet_url).await?;
    Ok(Json(ImportReportResponse::from(report)))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_game_order))
        .routes(routes!(update_game_order))
        .routes(routes!(delete_game_order))
        .routes(routes!(create_movie_order))
        .routes(routes!(update_movie_order))
        .routes(routes!(delete_movie_order))
        .routes(routes!(list_vip_records))
        .routes(routes!(vip_reminders))
        .routes(routes!(create_vip_record))
        .routes(routes!(update_vip_record))
        .routes(routes!(delete_vip_record))
        .routes(routes!(sheets_status))
        .routes(routes!(import_orders))
        .routes(routes!(sync_orders))
}

#[cfg(test)]
#[path = "orders.test.rs"]
mod tests;
