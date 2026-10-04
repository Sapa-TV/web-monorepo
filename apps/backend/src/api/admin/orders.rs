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
use crate::sheets::service::{ImportCount, ImportReport};
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
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct SheetsStatusResponse {
    pub configured: bool,
    pub spreadsheet_id: String,
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

#[utoipa::path(get, path = "/admin/orders/sheets", tag = "admin",
    responses((status = 200, description = "Sheets integration status", body = SheetsStatusResponse)))]
pub async fn sheets_status(State(state): State<AppState>) -> Json<SheetsStatusResponse> {
    Json(SheetsStatusResponse {
        configured: state.sheets.configured(),
        spreadsheet_id: state.sheets.spreadsheet_id(),
        _sealed: (),
    })
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
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::state::AppState;
    use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

    async fn admin_cookie(state: &AppState) -> String {
        state.admin_service.add("123", None).await.unwrap();
        session_cookie(state, "123").await
    }

    async fn request(
        app: &axum::Router,
        method: &str,
        uri: String,
        cookie: Option<&str>,
        body: Option<String>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        let body = match body {
            Some(json) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(json)
            }
            None => Body::empty(),
        };
        let response = app
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }

    #[tokio::test]
    async fn order_routes_require_admin() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let (status, _) = request(&app, "GET", api_path("/admin/orders/vip"), None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let cookie = session_cookie(&state, "999").await;
        let (status, _) = request(
            &app,
            "GET",
            api_path("/admin/orders/vip"),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_can_crud_game_orders() {
        let state = test_state().await;
        let app = test_router(state.clone());
        let cookie = admin_cookie(&state).await;

        let (status, body) = request(
            &app,
            "POST",
            api_path("/admin/orders/games"),
            Some(&cookie),
            Some(r#"{"title":null,"customer_name":"user_three","kind":"stream","source":"roulette","status":"pending","completed_at":null,"comment":null}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let id = body["id"].as_u64().unwrap();
        assert_eq!(body["title"], Value::Null);
        assert_eq!(body["customer_name"], "user_three");

        let (status, body) = request(
            &app,
            "PUT",
            api_path(&format!("/admin/orders/games/{id}")),
            Some(&cookie),
            Some(r#"{"title":"Test Game","customer_name":"user_three","kind":"stream","source":"roulette","status":"completed","completed_at":"2026-10-04","comment":"done"}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["title"], "Test Game");
        assert_eq!(body["status"], "completed");
        assert_eq!(body["completed_at"], "2026-10-04");

        let (status, _) = request(
            &app,
            "DELETE",
            api_path(&format!("/admin/orders/games/{id}")),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let (status, _) = request(
            &app,
            "PUT",
            api_path(&format!("/admin/orders/games/{id}")),
            Some(&cookie),
            Some(r#"{"title":null,"customer_name":"x","kind":"stream","source":"other","status":"pending","completed_at":null,"comment":null}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn admin_can_crud_movie_orders() {
        let state = test_state().await;
        let app = test_router(state.clone());
        let cookie = admin_cookie(&state).await;

        let (status, body) = request(
            &app,
            "POST",
            api_path("/admin/orders/movies"),
            Some(&cookie),
            Some(r#"{"title":"Фильм тест","customer_name":"Тест Заказчиков","kind":"movie","source":"donate","status":"pending","comment":null}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let id = body["id"].as_u64().unwrap();
        assert_eq!(body["kind"], "movie");

        let (status, body) = request(
            &app,
            "PUT",
            api_path(&format!("/admin/orders/movies/{id}")),
            Some(&cookie),
            Some(r#"{"title":"Фильм тест","customer_name":"Тест Заказчиков","kind":"movie","source":"donate","status":"completed","comment":"ok"}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "completed");

        let (status, _) = request(
            &app,
            "DELETE",
            api_path(&format!("/admin/orders/movies/{id}")),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_can_crud_vip_records_with_default_end_date() {
        let state = test_state().await;
        let app = test_router(state.clone());
        let cookie = admin_cookie(&state).await;

        let (status, body) = request(
            &app,
            "POST",
            api_path("/admin/orders/vip"),
            Some(&cookie),
            Some(r#"{"customer_name":"vip_user","kind":"vip","roulette_date":"2026-10-01","end_date":null,"note":null}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let id = body["id"].as_u64().unwrap();
        assert_eq!(body["end_date"], "2026-10-15");
        assert_eq!(body["status"], "active");

        let (status, body) = request(
            &app,
            "PUT",
            api_path(&format!("/admin/orders/vip/{id}")),
            Some(&cookie),
            Some(r#"{"customer_name":"vip_user","kind":"vip","roulette_date":"2026-10-01","end_date":"2026-10-15","status":"done","note":null}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "done");

        let (status, _) = request(
            &app,
            "POST",
            api_path("/admin/orders/vip"),
            Some(&cookie),
            Some(r#"{"customer_name":"x","kind":"unvip","roulette_date":"2026-10-10","end_date":"2026-10-01","note":null}"#.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, body) = request(
            &app,
            "GET",
            api_path("/admin/orders/vip"),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_array().unwrap().len(), 1);

        let (status, _) = request(
            &app,
            "DELETE",
            api_path(&format!("/admin/orders/vip/{id}")),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn vip_reminders_reflect_dates() {
        let state = test_state().await;
        let app = test_router(state.clone());
        let cookie = admin_cookie(&state).await;

        let today = chrono::Utc::now().date_naive();
        let soon = today + chrono::Duration::days(2);
        let past = today - chrono::Duration::days(1);
        let start = today - chrono::Duration::days(10);

        let body = format!(
            r#"{{"customer_name":"soon","kind":"vip","roulette_date":"{start}","end_date":"{soon}","note":null}}"#
        );
        request(
            &app,
            "POST",
            api_path("/admin/orders/vip"),
            Some(&cookie),
            Some(body),
        )
        .await;

        let body = format!(
            r#"{{"customer_name":"loser","kind":"unvip","roulette_date":"{start}","end_date":"{past}","note":null}}"#
        );
        request(
            &app,
            "POST",
            api_path("/admin/orders/vip"),
            Some(&cookie),
            Some(body),
        )
        .await;

        let (status, body) = request(
            &app,
            "GET",
            api_path("/admin/orders/vip/reminders?days=3"),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["expiring"].as_array().unwrap().len(), 1);
        assert_eq!(body["expiring"][0]["customer_name"], "soon");
        assert_eq!(body["awaiting_return"].as_array().unwrap().len(), 1);
        assert_eq!(body["awaiting_return"][0]["customer_name"], "loser");
    }
}
