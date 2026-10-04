use crate::db::sqlite::test_pool;
use crate::error::RepositoryError;
use crate::roulette::rarity::RarityId;
use crate::roulette::slot_service::RouletteSlotId;

use super::*;

async fn repo() -> SqliteRouletteSlotRepository {
    let (pool, _path) = test_pool().await;
    SqliteRouletteSlotRepository::new(pool)
}

#[tokio::test]
async fn load_all_returns_seeded_slots_in_order() {
    let repo = repo().await;

    let all = repo.load_all().await.unwrap();

    assert_eq!(all.len(), 4);
    assert_eq!(all[0].name, "Поболтать");
    assert_eq!(all[0].weight, 50);
    assert_eq!(all[3].name, "Джекпот");
    assert_eq!(all[3].weight, 1);
    assert_eq!(all[3].rarity_id, RarityId::new(4));
}

#[tokio::test]
async fn save_assigns_next_id_and_roundtrips() {
    let repo = repo().await;

    let saved = repo
        .save(RouletteSlot::new(
            RouletteSlotId::new(999),
            "Новый",
            RarityId::new(2),
            7,
            "chat",
        ))
        .await
        .unwrap();

    assert_eq!(saved.id.get(), 5);

    let all = repo.load_all().await.unwrap();
    let fetched = all.iter().find(|s| s.id == saved.id).unwrap();
    assert_eq!(fetched.name, "Новый");
    assert_eq!(fetched.weight, 7);
}

#[tokio::test]
async fn update_changes_fields_and_missing_is_none() {
    let repo = repo().await;

    let updated = repo
        .update(RouletteSlot::new(
            RouletteSlotId::new(1),
            "Поболтать",
            RarityId::new(3),
            99,
            "superchat",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.weight, 99);
    assert_eq!(updated.rarity_id, RarityId::new(3));

    assert!(
        repo.update(RouletteSlot::new(
            RouletteSlotId::new(999),
            "x",
            RarityId::new(1),
            1,
            "chat"
        ))
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn delete_reports_presence() {
    let repo = repo().await;

    assert!(repo.delete(RouletteSlotId::new(4)).await.unwrap());
    assert!(!repo.delete(RouletteSlotId::new(4)).await.unwrap());
}

#[tokio::test]
async fn weight_above_i64_max_is_conflict_not_panic() {
    let repo = repo().await;

    let err = repo
        .save(RouletteSlot::new(
            RouletteSlotId::new(999),
            "huge",
            RarityId::new(1),
            (i64::MAX as u64) + 1,
            "chat",
        ))
        .await
        .unwrap_err();

    match err {
        RepositoryError::Conflict(m) => assert_eq!(m, "weight out of range"),
        other => panic!("expected Conflict, got {other:?}"),
    }

    let err_update = repo
        .update(RouletteSlot::new(
            RouletteSlotId::new(1),
            "huge",
            RarityId::new(1),
            u64::MAX,
            "chat",
        ))
        .await
        .unwrap_err();
    assert!(matches!(err_update, RepositoryError::Conflict(_)));

    assert_eq!(repo.load_all().await.unwrap().len(), 4);
}
