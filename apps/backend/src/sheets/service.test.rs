use chrono::NaiveDate;

use super::*;
use crate::config::runtime::RuntimeConfig;
use crate::config::static_config::StaticConfig;
use crate::db::inmemory_config::InMemoryConfigRepository;
use crate::db::inmemory_orders::{
    InMemoryGameOrderRepository, InMemoryMovieOrderRepository, InMemoryVipRecordRepository,
};
use crate::orders::game::{GameOrderId, GameOrderKind, OrderSource};
use crate::orders::status::OrderStatus;
use crate::orders::vip::{VipKind, VipRecordId};

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
    let parsed = vec![game(Some("Test Game"), "user_one"), game(None, "user_one")];
    assert_eq!(new_games_only(&existing, parsed).len(), 2);
}

#[tokio::test]
async fn import_games_counts_skipped_duplicates() {
    let svc = test_service();
    svc.import_games(vec![game(Some("Test Game"), "user_one")])
        .await
        .unwrap();

    let count = svc
        .import_games(vec![
            game(Some("Test Game"), "user_one"),
            game(Some("Test Game"), "user_three"),
        ])
        .await
        .unwrap();
    assert_eq!(count, ImportCount::new(1, 1));
}

#[tokio::test]
async fn import_vip_dedupes_by_customer_kind_and_date() {
    let svc = test_service();
    let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let record = || NewVipRecord::new("vip_user".to_string(), None, VipKind::Vip, date, None, None);

    let first = svc.import_vip(vec![record()]).await.unwrap();
    assert_eq!(first, ImportCount::new(1, 0));

    let second = svc.import_vip(vec![record()]).await.unwrap();
    assert_eq!(second, ImportCount::new(0, 1));
}

#[tokio::test]
async fn import_vip_marks_expired_records_done() {
    let svc = test_service();
    let past = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let record = NewVipRecord::new(
        "old".to_string(),
        None,
        VipKind::Vip,
        past,
        Some(past),
        None,
    );

    svc.import_vip(vec![record]).await.unwrap();

    let all = svc.vip_repo.list().await.unwrap();
    assert_eq!(all[0].status, VipStatus::Done);
}

fn sample_game() -> GameOrder {
    let now = Utc::now();
    GameOrder::new(
        GameOrderId::new(7),
        Some("Test Game".to_string()),
        "user_one".to_string(),
        None,
        GameOrderKind::Playthrough,
        OrderSource::Roulette,
        OrderStatus::Completed,
        NaiveDate::from_ymd_opt(2026, 10, 2),
        None,
        now,
        now,
    )
}

#[test]
fn build_game_rows_renders_header_and_labels() {
    let rows = build_game_rows(&[sample_game()]);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][0], "№");
    assert_eq!(rows[0][6], "Дата окончания");
    assert_eq!(
        rows[1],
        vec![
            "7",
            "Test Game",
            "user_one",
            "прохождение",
            "рулетка",
            "пройдена",
            "2026-10-02",
            "",
        ]
    );
}

#[test]
fn build_vip_rows_formats_dates_and_status() {
    let now = Utc::now();
    let record = VipRecord::new(
        VipRecordId::new(3),
        "vip_user".to_string(),
        None,
        VipKind::Unvip,
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 10, 8).unwrap(),
        VipStatus::Active,
        None,
        now,
        now,
    );

    let rows = build_vip_rows(&[record]);

    assert_eq!(rows[0][1], "Тип");
    assert_eq!(
        rows[1],
        vec![
            "3",
            "UnVIP",
            "vip_user",
            "2026-10-01",
            "2026-10-08",
            "активна",
            ""
        ]
    );
}

#[tokio::test]
async fn import_without_client_is_not_configured() {
    let svc = test_service();
    let err = svc.import("whatever").await.unwrap_err();
    assert!(matches!(err, SheetsError::NotConfigured));
}
