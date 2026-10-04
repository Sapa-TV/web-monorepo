use crate::roulette::rarity::RarityId;

use super::*;

const COMMON: RarityId = RarityId::new(1);

fn make_slot(name: &str) -> RouletteSlot {
    RouletteSlot::new(RouletteSlotId::new(0), name, RarityId::new(1), 10, "action")
}

#[tokio::test]
async fn test_save_assigns_id() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let slot = make_slot("test");
    assert_eq!(slot.id, RouletteSlotId::new(0));

    let saved = repo.save(slot).await.unwrap();
    assert_eq!(saved.id, RouletteSlotId::new(1));
}

#[tokio::test]
async fn test_save_increments_id() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let saved_1 = repo.save(make_slot("a")).await.unwrap();
    let saved_2 = repo.save(make_slot("b")).await.unwrap();
    assert_eq!(saved_1.id, RouletteSlotId::new(1));
    assert_eq!(saved_2.id, RouletteSlotId::new(2));
}

#[tokio::test]
async fn test_load_all_returns_saved_slots() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    repo.save(make_slot("a")).await.unwrap();
    repo.save(make_slot("b")).await.unwrap();
    repo.save(make_slot("c")).await.unwrap();

    let all = repo.load_all().await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn test_update_existing_slot() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let saved = repo.save(make_slot("original")).await.unwrap();

    let updated = RouletteSlot::new(saved.id, "updated", COMMON, 99, "new action");
    let result = repo.update(updated.clone()).await.unwrap().unwrap();
    assert_eq!(result.name, "updated");
    assert_eq!(result.weight, 99);

    let all = repo.load_all().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "updated");
}

#[tokio::test]
async fn test_update_nonexistent_returns_none() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let slot = RouletteSlot::new(RouletteSlotId::new(999), "ghost", COMMON, 10, "action");
    let result = repo.update(slot).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_existing_slot() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let saved = repo.save(make_slot("to_delete")).await.unwrap();
    assert_eq!(repo.load_all().await.unwrap().len(), 1);

    repo.delete(saved.id).await.unwrap();
    assert_eq!(repo.load_all().await.unwrap().len(), 0);
}

#[tokio::test]
async fn test_delete_nonexistent_returns_false() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let result = repo.delete(RouletteSlotId::new(42)).await.unwrap();
    assert!(!result);
}

#[tokio::test]
async fn test_seed_load_all() {
    let slots = vec![make_slot("preloaded_a"), make_slot("preloaded_b")];
    let repo = InMemoryRouletteSlotRepository::seed(slots);
    let all = repo.load_all().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, RouletteSlotId::new(1));
    assert_eq!(all[1].id, RouletteSlotId::new(2));
    assert_eq!(all[0].name, "preloaded_a");
}

#[tokio::test]
async fn test_seed_continues_ids() {
    let slots = vec![make_slot("existing")];
    let repo = InMemoryRouletteSlotRepository::seed(slots);

    let saved = repo.save(make_slot("new")).await.unwrap();
    assert_eq!(saved.id, RouletteSlotId::new(2));
}

#[tokio::test]
async fn test_seed_normalizes_ids() {
    let s1 = RouletteSlot::new(RouletteSlotId::new(0), "a", COMMON, 10, "act");
    let s2 = RouletteSlot::new(RouletteSlotId::new(0), "b", COMMON, 10, "act");
    let repo = InMemoryRouletteSlotRepository::seed(vec![s1, s2]);

    let all = repo.load_all().await.unwrap();
    assert_eq!(all[0].id, RouletteSlotId::new(1));
    assert_eq!(all[1].id, RouletteSlotId::new(2));
}
