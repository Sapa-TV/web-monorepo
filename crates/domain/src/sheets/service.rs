use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::error::SheetsError;
use crate::orders::service::{
    GameOrderFilter, GameOrderService, MovieOrderFilter, MovieOrderService, VipRecordFilter,
    VipService,
};
use crate::sheets::client::SheetsClient;
use crate::sheets::parse;
use crate::sheets::settings::SheetsSettings;
use model::orders::game::{GameOrder, GameOrderKind, NewGameOrder, OrderSource};
use model::orders::movie::{MovieKind, MovieOrder, NewMovieOrder};
use model::orders::repository::{OrderRepository, VipRecordRepository};
use model::orders::status::OrderStatus;
use model::orders::vip::{NewVipRecord, VipKind, VipRecord, VipStatus};

const GAMES_RANGE: &str = "'Игры'!A1:H1000";
const MOVIES_RANGE: &str = "'Фильмы'!A1:H1000";
const VIP_RANGE: &str = "'VIP\\Unvip'!A1:H1000";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportCount {
    pub imported: u32,
    pub skipped: u32,
}

impl ImportCount {
    pub fn new(imported: u32, skipped: u32) -> Self {
        Self { imported, skipped }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ImportReport {
    pub spreadsheet_id: String,
    pub games: ImportCount,
    pub movies: ImportCount,
    pub vip: ImportCount,
    _sealed: (),
}

impl ImportReport {
    pub fn new(
        spreadsheet_id: String,
        games: ImportCount,
        movies: ImportCount,
        vip: ImportCount,
    ) -> Self {
        Self {
            spreadsheet_id,
            games,
            movies,
            vip,
            _sealed: (),
        }
    }
}

pub fn order_key(title: Option<&str>, customer: &str) -> String {
    format!(
        "{}|{}",
        title.unwrap_or("").to_lowercase(),
        customer.to_lowercase()
    )
}

pub fn new_games_only(existing: &[GameOrder], parsed: Vec<NewGameOrder>) -> Vec<NewGameOrder> {
    let keys: Vec<String> = existing
        .iter()
        .map(|o| order_key(o.title.as_deref(), &o.customer_name))
        .collect();
    parsed
        .into_iter()
        .filter(|o| !keys.contains(&order_key(o.title.as_deref(), &o.customer_name)))
        .collect()
}

pub fn new_movies_only(existing: &[MovieOrder], parsed: Vec<NewMovieOrder>) -> Vec<NewMovieOrder> {
    let keys: Vec<String> = existing
        .iter()
        .map(|o| order_key(o.title.as_deref(), &o.customer_name))
        .collect();
    parsed
        .into_iter()
        .filter(|o| !keys.contains(&order_key(o.title.as_deref(), &o.customer_name)))
        .collect()
}

pub fn vip_key(customer: &str, kind: VipKind, date: chrono::NaiveDate) -> String {
    format!("{}|{kind:?}|{date}", customer.to_lowercase())
}

pub fn new_vip_only(existing: &[VipRecord], parsed: Vec<NewVipRecord>) -> Vec<NewVipRecord> {
    let keys: Vec<String> = existing
        .iter()
        .map(|r| vip_key(&r.customer_name, r.kind, r.roulette_date))
        .collect();
    parsed
        .into_iter()
        .filter(|r| !keys.contains(&vip_key(&r.customer_name, r.kind, r.roulette_date)))
        .collect()
}

#[non_exhaustive]
pub struct SheetsService<C, G, M, V, S>
where
    G: OrderRepository<Order = GameOrder, New = NewGameOrder, Kind = GameOrderKind>,
    M: OrderRepository<Order = MovieOrder, New = NewMovieOrder, Kind = MovieKind>,
    V: VipRecordRepository,
    C: SheetsClient,
    S: SheetsSettings,
{
    client: Option<C>,
    pub game_service: Arc<GameOrderService<G>>,
    pub movie_service: Arc<MovieOrderService<M>>,
    pub vip_service: Arc<VipService<V>>,
    settings: Arc<S>,
    dirty: AtomicBool,
    last_sync: Mutex<Option<DateTime<Utc>>>,
}

impl<C, G, M, V, S> SheetsService<C, G, M, V, S>
where
    G: OrderRepository<Order = GameOrder, New = NewGameOrder, Kind = GameOrderKind>,
    M: OrderRepository<Order = MovieOrder, New = NewMovieOrder, Kind = MovieKind>,
    V: VipRecordRepository,
    C: SheetsClient,
    S: SheetsSettings,
{
    pub fn new(
        client: Option<C>,
        game_service: Arc<GameOrderService<G>>,
        movie_service: Arc<MovieOrderService<M>>,
        vip_service: Arc<VipService<V>>,
        settings: Arc<S>,
    ) -> Self {
        Self {
            client,
            game_service,
            movie_service,
            vip_service,
            settings,
            dirty: AtomicBool::new(false),
            last_sync: Mutex::new(None),
        }
    }

    pub fn configured(&self) -> bool {
        self.client.is_some()
    }

    pub fn spreadsheet_id(&self) -> String {
        self.settings.sheets_spreadsheet_id()
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn last_synced_at(&self) -> Option<DateTime<Utc>> {
        *self.last_sync.lock().expect("last_sync poisoned")
    }

    pub async fn import(&self, spreadsheet_url: &str) -> Result<ImportReport, SheetsError> {
        let client = self.client.as_ref().ok_or(SheetsError::NotConfigured)?;
        let spreadsheet_id = parse::parse_spreadsheet_id(spreadsheet_url)
            .ok_or(SheetsError::InvalidSpreadsheetUrl)?;

        let games_values = client.get_values(&spreadsheet_id, GAMES_RANGE).await?;
        let movies_values = client.get_values(&spreadsheet_id, MOVIES_RANGE).await?;
        let vip_values = client.get_values(&spreadsheet_id, VIP_RANGE).await?;

        let parsed_games = parse::parse_game_rows(&games_values);
        let parsed_movies = parse::parse_movie_rows(&movies_values);
        let parsed_vip = parse::parse_vip_rows(&vip_values);

        let games = self.import_games(parsed_games).await?;
        let movies = self.import_movies(parsed_movies).await?;
        let vip = self.import_vip(parsed_vip).await?;

        self.settings
            .set_sheets_spreadsheet_id(&spreadsheet_id)
            .await
            .map_err(|e| SheetsError::Config(e.to_string()))?;

        Ok(ImportReport::new(spreadsheet_id, games, movies, vip))
    }

    pub async fn import_games(
        &self,
        parsed: Vec<NewGameOrder>,
    ) -> Result<ImportCount, SheetsError> {
        let total = parsed.len() as u32;
        let existing = self.game_service.list(GameOrderFilter::default()).await?;
        let fresh = new_games_only(&existing, parsed);
        let imported = fresh.len() as u32;
        for order in fresh {
            self.game_service.create(order).await?;
        }
        Ok(ImportCount::new(imported, total - imported))
    }

    pub async fn import_movies(
        &self,
        parsed: Vec<NewMovieOrder>,
    ) -> Result<ImportCount, SheetsError> {
        let total = parsed.len() as u32;
        let existing = self.movie_service.list(MovieOrderFilter::default()).await?;
        let fresh = new_movies_only(&existing, parsed);
        let imported = fresh.len() as u32;
        for order in fresh {
            self.movie_service.create(order).await?;
        }
        Ok(ImportCount::new(imported, total - imported))
    }

    pub async fn import_vip(&self, parsed: Vec<NewVipRecord>) -> Result<ImportCount, SheetsError> {
        let total = parsed.len() as u32;
        let existing = self.vip_service.list(VipRecordFilter::default()).await?;
        let fresh = new_vip_only(&existing, parsed);
        let imported = fresh.len() as u32;
        let today = chrono::Utc::now().date_naive();
        for record in fresh {
            let mut created = self.vip_service.create(record).await?;
            if created.end_date < today {
                created.status = VipStatus::Done;
                self.vip_service.update(created).await?;
            }
        }
        Ok(ImportCount::new(imported, total - imported))
    }
}

const GAMES_CLEAR_RANGE: &str = "'Игры'!A1:H2000";
const MOVIES_CLEAR_RANGE: &str = "'Фильмы'!A1:G2000";
const VIP_CLEAR_RANGE: &str = "'VIP\\Unvip'!A1:G2000";

pub const SYNC_FULL_INTERVAL_SECS: u64 = 24 * 60 * 60;
pub const SYNC_CHECK_INTERVAL_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SyncReport {
    pub games: u32,
    pub movies: u32,
    pub vip: u32,
    pub synced_at: DateTime<Utc>,
    _sealed: (),
}

impl SyncReport {
    pub fn new(games: u32, movies: u32, vip: u32, synced_at: DateTime<Utc>) -> Self {
        Self {
            games,
            movies,
            vip,
            synced_at,
            _sealed: (),
        }
    }
}

fn game_kind_label(kind: GameOrderKind) -> &'static str {
    match kind {
        GameOrderKind::Stream => "стрим",
        GameOrderKind::Playthrough => "прохождение",
        _ => "",
    }
}

fn movie_kind_label(kind: MovieKind) -> &'static str {
    match kind {
        MovieKind::Movie => "фильм",
        MovieKind::Series => "сериал",
        MovieKind::Anime => "аниме",
        MovieKind::Youtube => "ютуб",
        _ => "",
    }
}

fn source_label(source: OrderSource) -> &'static str {
    match source {
        OrderSource::Donate => "донат",
        OrderSource::Points => "баллы",
        OrderSource::Roulette => "рулетка",
        OrderSource::Other => "",
        _ => "",
    }
}

fn order_status_label(status: OrderStatus, completed: &str) -> &str {
    match status {
        OrderStatus::Pending => "ожидает",
        OrderStatus::Completed => completed,
        OrderStatus::Cancelled => "отмена",
        _ => "",
    }
}

fn vip_kind_label(kind: VipKind) -> &'static str {
    match kind {
        VipKind::Vip => "VIP",
        VipKind::Unvip => "UnVIP",
        _ => "",
    }
}

fn vip_status_label(status: VipStatus) -> &'static str {
    match status {
        VipStatus::Active => "активна",
        VipStatus::Done => "завершена",
        VipStatus::Cancelled => "отменена",
        _ => "",
    }
}

fn opt_str(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

pub fn build_game_rows(orders: &[GameOrder]) -> Vec<Vec<String>> {
    let mut rows = vec![vec![
        "№".into(),
        "Игра".into(),
        "Заказчик".into(),
        "Тип".into(),
        "Источник".into(),
        "Статус".into(),
        "Дата окончания".into(),
        "Комментарий".into(),
    ]];
    for o in orders {
        rows.push(vec![
            o.id.get().to_string(),
            opt_str(&o.title),
            o.customer_name.clone(),
            game_kind_label(o.kind).into(),
            source_label(o.source).into(),
            order_status_label(o.status, "пройдена").into(),
            o.completed_at.map(|d| d.to_string()).unwrap_or_default(),
            opt_str(&o.comment),
        ]);
    }
    rows
}

pub fn build_movie_rows(orders: &[MovieOrder]) -> Vec<Vec<String>> {
    let mut rows = vec![vec![
        "№".into(),
        "Название".into(),
        "Заказчик".into(),
        "Тип".into(),
        "Источник".into(),
        "Статус".into(),
        "Комментарий".into(),
    ]];
    for o in orders {
        rows.push(vec![
            o.id.get().to_string(),
            opt_str(&o.title),
            o.customer_name.clone(),
            movie_kind_label(o.kind).into(),
            source_label(o.source).into(),
            order_status_label(o.status, "просмотрен").into(),
            opt_str(&o.comment),
        ]);
    }
    rows
}

pub fn build_vip_rows(records: &[VipRecord]) -> Vec<Vec<String>> {
    let mut rows = vec![vec![
        "№".into(),
        "Тип".into(),
        "Заказчик".into(),
        "Дата рулетки".into(),
        "Дата окончания".into(),
        "Статус".into(),
        "Заметка".into(),
    ]];
    for r in records {
        rows.push(vec![
            r.id.get().to_string(),
            vip_kind_label(r.kind).into(),
            r.customer_name.clone(),
            r.roulette_date.to_string(),
            r.end_date.to_string(),
            vip_status_label(r.status).into(),
            opt_str(&r.note),
        ]);
    }
    rows
}

impl<C, G, M, V, S> SheetsService<C, G, M, V, S>
where
    G: OrderRepository<Order = GameOrder, New = NewGameOrder, Kind = GameOrderKind>,
    M: OrderRepository<Order = MovieOrder, New = NewMovieOrder, Kind = MovieKind>,
    V: VipRecordRepository,
    C: SheetsClient,
    S: SheetsSettings,
{
    async fn client_and_spreadsheet(&self) -> Result<(&C, String), SheetsError> {
        let client = self.client.as_ref().ok_or(SheetsError::NotConfigured)?;
        let spreadsheet_id = self.spreadsheet_id();
        if spreadsheet_id.is_empty() {
            return Err(SheetsError::MissingSpreadsheetId);
        }
        Ok((client, spreadsheet_id))
    }

    pub async fn sync(&self) -> Result<SyncReport, SheetsError> {
        let (client, spreadsheet_id) = self.client_and_spreadsheet().await?;

        let games = self
            .game_service
            .list(GameOrderFilter::default())
            .await?
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let movies = self
            .movie_service
            .list(MovieOrderFilter::default())
            .await?
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let vip = self
            .vip_service
            .list(VipRecordFilter::default())
            .await?
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        client
            .clear_values(&spreadsheet_id, GAMES_CLEAR_RANGE)
            .await?;
        client
            .update_values(&spreadsheet_id, GAMES_CLEAR_RANGE, build_game_rows(&games))
            .await?;
        client
            .clear_values(&spreadsheet_id, MOVIES_CLEAR_RANGE)
            .await?;
        client
            .update_values(
                &spreadsheet_id,
                MOVIES_CLEAR_RANGE,
                build_movie_rows(&movies),
            )
            .await?;
        client
            .clear_values(&spreadsheet_id, VIP_CLEAR_RANGE)
            .await?;
        client
            .update_values(&spreadsheet_id, VIP_CLEAR_RANGE, build_vip_rows(&vip))
            .await?;

        let synced_at = Utc::now();
        *self.last_sync.lock().expect("last_sync poisoned") = Some(synced_at);
        self.dirty.store(false, Ordering::Relaxed);

        Ok(SyncReport::new(
            games.len() as u32,
            movies.len() as u32,
            vip.len() as u32,
            synced_at,
        ))
    }

    /// Background hook: syncs when data changed or the last sync is older than a day.
    pub async fn sync_if_due(&self) -> Result<bool, SheetsError> {
        if !self.configured() {
            return Ok(false);
        }
        let dirty = self.dirty.swap(false, Ordering::Relaxed);
        let stale = self
            .last_synced_at()
            .is_some_and(|t| (Utc::now() - t).num_seconds() as u64 >= SYNC_FULL_INTERVAL_SECS);
        if !dirty && !stale {
            return Ok(false);
        }
        match self.sync().await {
            Ok(_) => Ok(true),
            Err(e) => {
                if dirty {
                    self.dirty.store(true, Ordering::Relaxed);
                }
                Err(e)
            }
        }
    }
}
