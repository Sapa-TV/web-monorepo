use crate::db::sqlite::test_pool;

use super::*;

async fn repo() -> SqliteRarityRepository {
    let (pool, _path) = test_pool().await;
    SqliteRarityRepository::new(pool)
}

#[tokio::test]
async fn load_all_returns_seeded_four_in_order() {
    let repo = repo().await;

    let all = repo.load_all().await.unwrap();

    let ids: Vec<u32> = all.iter().map(|r| r.id.get()).collect();
    assert_eq!(ids, vec![1, 2, 3, 4]);
    assert_eq!(all[0].name, "common");
    assert_eq!(all[3].name, "legendary");
}

#[tokio::test]
async fn save_assigns_next_id_after_seed() {
    let repo = repo().await;

    let saved = repo
        .save(Rarity::new(
            RarityId::new(0),
            "mythic",
            "Mythic",
            "mythic.png",
            "#733f88",
        ))
        .await
        .unwrap();

    assert_eq!(saved.id.get(), 5);
    assert_eq!(saved.name, "mythic");
}

#[tokio::test]
async fn update_roundtrip_and_missing_is_none() {
    let repo = repo().await;

    let updated = repo
        .update(Rarity::new(
            RarityId::new(1),
            "common",
            "Common Renamed",
            "common.png",
            "#ffffff",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.display_name, "Common Renamed");
    assert_eq!(updated.id.get(), 1);

    assert!(
        repo.update(Rarity::new(
            RarityId::new(999),
            "x",
            "X",
            "x.png",
            "#000000"
        ))
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn delete_reports_presence() {
    let repo = repo().await;

    assert!(repo.delete(RarityId::new(4)).await.unwrap());
    assert!(!repo.delete(RarityId::new(4)).await.unwrap());
}

#[tokio::test]
async fn deleting_rarity_cascades_to_its_slots() {
    use crate::db::sqlite::roulette_slot::SqliteRouletteSlotRepository;
    use crate::roulette::repository::RouletteSlotRepository;

    let (pool, _path) = test_pool().await;
    let repo = SqliteRarityRepository::new(pool.clone());
    let slots = SqliteRouletteSlotRepository::new(pool);

    assert_eq!(slots.load_all().await.unwrap().len(), 4);

    assert!(repo.delete(RarityId::new(4)).await.unwrap());

    let remaining = slots.load_all().await.unwrap();
    assert_eq!(remaining.len(), 3);
    assert!(remaining.iter().all(|s| s.name != "Джекпот"));
}
