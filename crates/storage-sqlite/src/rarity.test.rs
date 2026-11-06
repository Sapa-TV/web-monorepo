use crate::test_pool;

use super::*;

async fn repo() -> SqliteRarityRepository {
    let (pool, _path) = test_pool().await;
    SqliteRarityRepository::new(pool)
}

fn rarity(name: &str) -> Rarity {
    Rarity::new(
        RarityId::new(0),
        name.to_string(),
        name.to_string(),
        format!("{name}.png"),
        "#000000".to_string(),
    )
}

#[tokio::test]
async fn load_all_orders_by_id_and_save_appends() {
    let repo = repo().await;
    let max = repo
        .load_all()
        .await
        .unwrap()
        .iter()
        .map(|r| r.id.get())
        .max()
        .unwrap();

    let saved = repo.save(rarity("aaa")).await.unwrap();

    assert_eq!(saved.id.get(), max + 1);

    let ids: Vec<u32> = repo
        .load_all()
        .await
        .unwrap()
        .iter()
        .map(|r| r.id.get())
        .collect();
    assert_eq!(ids.last(), Some(&saved.id.get()));
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
}

#[tokio::test]
async fn update_roundtrip_and_missing_is_none() {
    let repo = repo().await;
    let saved = repo.save(rarity("custom")).await.unwrap();

    let updated = repo
        .update(Rarity::new(
            saved.id,
            "custom",
            "Custom Renamed",
            "custom.png",
            "#000000",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.display_name, "Custom Renamed");
    assert_eq!(updated.id, saved.id);

    let missing = RarityId::new(saved.id.get() + 10_000);
    assert!(
        repo.update(Rarity::new(missing, "x", "X", "x.png", "#ffffff"))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn delete_reports_presence() {
    let repo = repo().await;
    let saved = repo.save(rarity("temp")).await.unwrap();

    assert!(repo.delete(saved.id).await.unwrap());
    assert!(!repo.delete(saved.id).await.unwrap());
}

#[tokio::test]
async fn deleting_rarity_cascades_to_its_slots() {
    use crate::roulette_slot::SqliteRouletteSlotRepository;
    use model::roulette::repository::RouletteSlotRepository;
    use model::roulette::slot::{RouletteSlot, RouletteSlotId};

    let (pool, _path) = test_pool().await;
    let repo = SqliteRarityRepository::new(pool.clone());
    let slots = SqliteRouletteSlotRepository::new(pool);

    let rarity = repo.save(rarity("cascade")).await.unwrap();
    for name in ["first", "second"] {
        slots
            .save(RouletteSlot::new(
                RouletteSlotId::new(0),
                name,
                rarity.id,
                1,
                "chat",
            ))
            .await
            .unwrap();
    }

    let before = slots.load_all().await.unwrap();
    let owned = before.iter().filter(|s| s.rarity_id == rarity.id).count();
    assert_eq!(owned, 2);

    assert!(repo.delete(rarity.id).await.unwrap());

    let after = slots.load_all().await.unwrap();
    assert_eq!(after.len(), before.len() - owned);
    assert!(after.iter().all(|s| s.rarity_id != rarity.id));
}
