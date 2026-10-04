use chrono::NaiveDate;

use super::*;
use crate::db::inmemory_orders::{
    InMemoryGameOrderRepository, InMemoryMovieOrderRepository, InMemoryVipRecordRepository,
};

fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn new_game(title: Option<&str>, customer: &str) -> NewGameOrder {
    NewGameOrder::new(
        title.map(str::to_string),
        customer.to_string(),
        None,
        GameOrderKind::Stream,
        OrderSource::Roulette,
        OrderStatus::Pending,
        None,
        None,
    )
}

#[tokio::test]
async fn game_create_trims_and_validates() {
    let svc = GameOrderService::new(InMemoryGameOrderRepository::new());

    let created = svc
        .create(new_game(Some("  "), "  test_user  "))
        .await
        .unwrap();
    assert_eq!(created.title, None);
    assert_eq!(created.customer_name, "test_user");

    let err = svc.create(new_game(Some("x"), "   ")).await.unwrap_err();
    assert!(matches!(err, OrdersServiceError::Invalid(_)));
}

#[tokio::test]
async fn game_list_filters_and_searches() {
    let svc = GameOrderService::new(InMemoryGameOrderRepository::new());
    svc.create(new_game(Some("Test Game"), "user_one"))
        .await
        .unwrap();
    let mut second = new_game(Some("Strategy Game"), "user_two");
    second.kind = GameOrderKind::Playthrough;
    second.source = OrderSource::Donate;
    svc.create(second).await.unwrap();
    let mut third = new_game(None, "user_three");
    third.status = OrderStatus::Cancelled;
    svc.create(third).await.unwrap();

    let pending = svc
        .list(GameOrderFilter {
            status: Some(OrderStatus::Pending),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(pending.len(), 2);

    let playthroughs = svc
        .list(GameOrderFilter {
            kind: Some(GameOrderKind::Playthrough),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(playthroughs.len(), 1);
    assert_eq!(playthroughs[0].customer_name, "user_two");

    let found = svc
        .list(GameOrderFilter {
            query: Some("strateg".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(found.len(), 1);

    let by_customer = svc
        .list(GameOrderFilter {
            query: Some("USER_THREE".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_customer.len(), 1);
    assert_eq!(by_customer[0].title, None);
}

#[tokio::test]
async fn movie_crud_roundtrip() {
    let svc = MovieOrderService::new(InMemoryMovieOrderRepository::new());
    let created = svc
        .create(NewMovieOrder::new(
            Some("Тестовый фильм".to_string()),
            "user_four".to_string(),
            None,
            MovieKind::Movie,
            OrderSource::Donate,
            OrderStatus::Pending,
            None,
        ))
        .await
        .unwrap();

    let mut updated = created.clone();
    updated.status = OrderStatus::Completed;
    let updated = svc.update(updated).await.unwrap();
    assert_eq!(updated.status, OrderStatus::Completed);

    svc.delete(created.id).await.unwrap();
    let err = svc.get_by_id(created.id).await.unwrap_err();
    assert!(matches!(err, OrdersServiceError::NotFound));
}

#[tokio::test]
async fn vip_default_end_dates_by_kind() {
    let svc = VipService::new(InMemoryVipRecordRepository::new());
    let start = date("2026-10-01");

    let vip = svc
        .create(NewVipRecord::new(
            "vip_user".to_string(),
            None,
            VipKind::Vip,
            start,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(vip.end_date, date("2026-10-15"));
    assert_eq!(vip.status, VipStatus::Active);

    let unvip = svc
        .create(NewVipRecord::new(
            "unvip_user".to_string(),
            None,
            VipKind::Unvip,
            start,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(unvip.end_date, date("2026-10-08"));

    let err = svc
        .create(NewVipRecord::new(
            "x".to_string(),
            None,
            VipKind::Vip,
            start,
            Some(date("2026-09-30")),
            None,
        ))
        .await
        .unwrap_err();
    assert!(matches!(err, OrdersServiceError::Invalid(_)));
}

#[tokio::test]
async fn vip_reminders() {
    let svc = VipService::new(InMemoryVipRecordRepository::new());
    let today = date("2026-10-10");

    svc.create(NewVipRecord::new(
        "soon".to_string(),
        None,
        VipKind::Vip,
        date("2026-09-30"),
        Some(date("2026-10-12")),
        None,
    ))
    .await
    .unwrap();
    svc.create(NewVipRecord::new(
        "far".to_string(),
        None,
        VipKind::Vip,
        date("2026-10-09"),
        Some(date("2026-10-30")),
        None,
    ))
    .await
    .unwrap();
    svc.create(NewVipRecord::new(
        "past".to_string(),
        None,
        VipKind::Vip,
        date("2026-09-01"),
        Some(date("2026-10-05")),
        None,
    ))
    .await
    .unwrap();
    svc.create(NewVipRecord::new(
        "loser".to_string(),
        None,
        VipKind::Unvip,
        date("2026-10-01"),
        Some(date("2026-10-08")),
        None,
    ))
    .await
    .unwrap();

    let expiring = svc.expiring_within(today, 3).await.unwrap();
    assert_eq!(expiring.len(), 1);
    assert_eq!(expiring[0].customer_name, "soon");

    let awaiting = svc.awaiting_return(today).await.unwrap();
    assert_eq!(awaiting.len(), 1);
    assert_eq!(awaiting[0].customer_name, "loser");
}
