use crate::rarity::SqliteRarityRepository;
use crate::test_pool;
use model::error::RepositoryError;
use model::roulette::rarity::{Rarity, RarityId, RarityRepository};
use model::roulette::slot::RouletteSlotId;

use super::*;

async fn setup() -> (SqliteRouletteSlotRepository, SqliteRarityRepository) {
    let (pool, _path) = test_pool().await;
    (
        SqliteRouletteSlotRepository::new(pool.clone()),
        SqliteRarityRepository::new(pool),
    )
}

async fn new_rarity(rarities: &SqliteRarityRepository, name: &str) -> RarityId {
    rarities
        .save(Rarity::new(
            RarityId::new(0),
            name,
            name,
            "rarity.png",
            "#000000",
        ))
        .await
        .unwrap()
        .id
}

fn slot(name: &str, rarity_id: RarityId, weight: u64) -> RouletteSlot {
    RouletteSlot::new(RouletteSlotId::new(0), name, rarity_id, weight, "chat")
}

#[tokio::test]
async fn seeded_slots_are_ordered_and_reference_existing_rarities() {
    let (slots, rarities) = setup().await;

    let all = slots.load_all().await.unwrap();
    assert!(!all.is_empty());

    let ids: Vec<u32> = all.iter().map(|s| s.id.get()).collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]));

    let known: Vec<RarityId> = rarities
        .load_all()
        .await
        .unwrap()
        .iter()
        .map(|r| r.id)
        .collect();
    assert!(all.iter().all(|s| known.contains(&s.rarity_id)));
}

#[tokio::test]
async fn save_assigns_next_id_and_roundtrips() {
    let (slots, rarities) = setup().await;
    let max = slots
        .load_all()
        .await
        .unwrap()
        .iter()
        .map(|s| s.id.get())
        .max()
        .unwrap();
    let rarity = new_rarity(&rarities, "custom").await;

    let saved = slots.save(slot("Новый", rarity, 7)).await.unwrap();

    assert_eq!(saved.id.get(), max + 1);

    let all = slots.load_all().await.unwrap();
    let fetched = all.iter().find(|s| s.id == saved.id).unwrap();
    assert_eq!(fetched.name, "Новый");
    assert_eq!(fetched.weight, 7);
    assert_eq!(fetched.rarity_id, rarity);
}

#[tokio::test]
async fn update_changes_fields_and_missing_is_none() {
    let (slots, rarities) = setup().await;
    let rarity = new_rarity(&rarities, "custom").await;
    let saved = slots.save(slot("before", rarity, 1)).await.unwrap();

    let updated = slots
        .update(RouletteSlot::new(
            saved.id,
            "after",
            rarity,
            99,
            "superchat",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.weight, 99);
    assert_eq!(updated.name, "after");

    let missing = RouletteSlotId::new(saved.id.get() + 10_000);
    assert!(
        slots
            .update(RouletteSlot::new(missing, "x", rarity, 1, "chat"))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn delete_reports_presence() {
    let (slots, rarities) = setup().await;
    let rarity = new_rarity(&rarities, "custom").await;
    let saved = slots.save(slot("temp", rarity, 1)).await.unwrap();

    assert!(slots.delete(saved.id).await.unwrap());
    assert!(!slots.delete(saved.id).await.unwrap());
}

#[tokio::test]
async fn weight_above_i64_max_is_conflict_not_panic() {
    let (slots, rarities) = setup().await;
    let rarity = new_rarity(&rarities, "custom").await;
    let saved = slots.save(slot("existing", rarity, 1)).await.unwrap();
    let before = slots.load_all().await.unwrap().len();

    let err = slots
        .save(RouletteSlot::new(
            RouletteSlotId::new(0),
            "huge",
            rarity,
            (i64::MAX as u64) + 1,
            "chat",
        ))
        .await
        .unwrap_err();

    match err {
        RepositoryError::Conflict(m) => assert_eq!(m, "weight out of range"),
        other => panic!("expected Conflict, got {other:?}"),
    }

    let err_update = slots
        .update(RouletteSlot::new(
            saved.id,
            "huge",
            rarity,
            u64::MAX,
            "chat",
        ))
        .await
        .unwrap_err();
    assert!(matches!(err_update, RepositoryError::Conflict(_)));

    assert_eq!(slots.load_all().await.unwrap().len(), before);
}
