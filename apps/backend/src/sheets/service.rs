use std::sync::Arc;

use crate::config::repository::ConfigRepository;
use crate::config::store::ConfigStore;
use crate::error::SheetsError;
use crate::orders::game::{GameOrder, NewGameOrder};
use crate::orders::movie::{MovieOrder, NewMovieOrder};
use crate::orders::repository::{GameOrderRepository, MovieOrderRepository, VipRecordRepository};
use crate::orders::vip::{NewVipRecord, VipKind, VipRecord};
use crate::sheets::client::GoogleSheetsClient;
use crate::sheets::parse;

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

fn order_key(title: Option<&str>, customer: &str) -> String {
    format!(
        "{}|{}",
        title.unwrap_or("").to_lowercase(),
        customer.to_lowercase()
    )
}

fn new_games_only(existing: &[GameOrder], parsed: Vec<NewGameOrder>) -> Vec<NewGameOrder> {
    let keys: Vec<String> = existing
        .iter()
        .map(|o| order_key(o.title.as_deref(), &o.customer_name))
        .collect();
    parsed
        .into_iter()
        .filter(|o| !keys.contains(&order_key(o.title.as_deref(), &o.customer_name)))
        .collect()
}

fn new_movies_only(existing: &[MovieOrder], parsed: Vec<NewMovieOrder>) -> Vec<NewMovieOrder> {
    let keys: Vec<String> = existing
        .iter()
        .map(|o| order_key(o.title.as_deref(), &o.customer_name))
        .collect();
    parsed
        .into_iter()
        .filter(|o| !keys.contains(&order_key(o.title.as_deref(), &o.customer_name)))
        .collect()
}

fn vip_key(customer: &str, kind: VipKind, date: chrono::NaiveDate) -> String {
    format!("{}|{kind:?}|{date}", customer.to_lowercase())
}

fn new_vip_only(existing: &[VipRecord], parsed: Vec<NewVipRecord>) -> Vec<NewVipRecord> {
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
pub struct SheetsService<G, M, V, K>
where
    G: GameOrderRepository,
    M: MovieOrderRepository,
    V: VipRecordRepository,
    K: ConfigRepository,
{
    client: Option<GoogleSheetsClient>,
    game_repo: G,
    movie_repo: M,
    vip_repo: V,
    config: Arc<ConfigStore<K>>,
}

impl<G, M, V, K> SheetsService<G, M, V, K>
where
    G: GameOrderRepository,
    M: MovieOrderRepository,
    V: VipRecordRepository,
    K: ConfigRepository,
{
    pub fn new(
        client: Option<GoogleSheetsClient>,
        game_repo: G,
        movie_repo: M,
        vip_repo: V,
        config: Arc<ConfigStore<K>>,
    ) -> Self {
        Self {
            client,
            game_repo,
            movie_repo,
            vip_repo,
            config,
        }
    }

    pub fn configured(&self) -> bool {
        self.client.is_some()
    }

    pub fn spreadsheet_id(&self) -> String {
        self.config.sheets_spreadsheet_id()
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

        self.config
            .set_sheets_spreadsheet_id(&spreadsheet_id)
            .await
            .map_err(|e| SheetsError::Config(e.to_string()))?;

        Ok(ImportReport::new(spreadsheet_id, games, movies, vip))
    }

    async fn import_games(&self, parsed: Vec<NewGameOrder>) -> Result<ImportCount, SheetsError> {
        let total = parsed.len() as u32;
        let fresh = new_games_only(&self.game_repo.list().await?, parsed);
        let imported = fresh.len() as u32;
        for order in fresh {
            self.game_repo.create(order).await?;
        }
        Ok(ImportCount::new(imported, total - imported))
    }

    async fn import_movies(&self, parsed: Vec<NewMovieOrder>) -> Result<ImportCount, SheetsError> {
        let total = parsed.len() as u32;
        let fresh = new_movies_only(&self.movie_repo.list().await?, parsed);
        let imported = fresh.len() as u32;
        for order in fresh {
            self.movie_repo.create(order).await?;
        }
        Ok(ImportCount::new(imported, total - imported))
    }

    async fn import_vip(&self, parsed: Vec<NewVipRecord>) -> Result<ImportCount, SheetsError> {
        let total = parsed.len() as u32;
        let fresh = new_vip_only(&self.vip_repo.list().await?, parsed);
        let imported = fresh.len() as u32;
        for record in fresh {
            let end_date = record.end_date.unwrap_or(record.roulette_date);
            self.vip_repo.create(record, end_date).await?;
        }
        Ok(ImportCount::new(imported, total - imported))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::config::runtime::RuntimeConfig;
    use crate::config::static_config::StaticConfig;
    use crate::db::inmemory_config::InMemoryConfigRepository;
    use crate::db::inmemory_orders::{
        InMemoryGameOrderRepository, InMemoryMovieOrderRepository, InMemoryVipRecordRepository,
    };
    use crate::orders::game::{GameOrderKind, OrderSource};
    use crate::orders::status::OrderStatus;
    use crate::orders::vip::VipKind;

    fn game(title: Option<&str>, customer: &str) -> NewGameOrder {
        NewGameOrder::new(
            title.map(str::to_string),
            customer.to_string(),
            None,
            GameOrderKind::Stream,
            OrderSource::Other,
            OrderStatus::Pending,
            None,
            None,
        )
    }

    fn test_service() -> SheetsService<
        InMemoryGameOrderRepository,
        InMemoryMovieOrderRepository,
        InMemoryVipRecordRepository,
        InMemoryConfigRepository,
    > {
        let config = Arc::new(ConfigStore::new(
            Arc::new(StaticConfig::test_config()),
            RuntimeConfig::test_runtime("test-key"),
            Arc::new(InMemoryConfigRepository::new()),
        ));
        SheetsService::new(
            None,
            InMemoryGameOrderRepository::new(),
            InMemoryMovieOrderRepository::new(),
            InMemoryVipRecordRepository::new(),
            config,
        )
    }

    #[test]
    fn dedup_games_by_title_and_customer() {
        let existing: Vec<GameOrder> = vec![];
        let parsed = vec![game(Some("Noita"), "Rikrims"), game(None, "Rikrims")];
        assert_eq!(new_games_only(&existing, parsed).len(), 2);
    }

    #[tokio::test]
    async fn import_games_counts_skipped_duplicates() {
        let svc = test_service();
        svc.import_games(vec![game(Some("Noita"), "Rikrims")])
            .await
            .unwrap();

        let count = svc
            .import_games(vec![
                game(Some("Noita"), "Rikrims"),
                game(Some("Noita"), "Jeker3"),
            ])
            .await
            .unwrap();
        assert_eq!(count, ImportCount::new(1, 1));
    }

    #[tokio::test]
    async fn import_vip_dedupes_by_customer_kind_and_date() {
        let svc = test_service();
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let record = || {
            NewVipRecord::new(
                "kasperaas".to_string(),
                None,
                VipKind::Vip,
                date,
                None,
                None,
            )
        };

        let first = svc.import_vip(vec![record()]).await.unwrap();
        assert_eq!(first, ImportCount::new(1, 0));

        let second = svc.import_vip(vec![record()]).await.unwrap();
        assert_eq!(second, ImportCount::new(0, 1));
    }

    #[tokio::test]
    async fn import_without_client_is_not_configured() {
        let svc = test_service();
        let err = svc.import("whatever").await.unwrap_err();
        assert!(matches!(err, SheetsError::NotConfigured));
    }
}
