use crate::db::inmemory_rarity::InMemoryRarityRepository;

use super::*;

#[tokio::test]
async fn build_loads_seeded() {
    let repo = InMemoryRarityRepository::new_seeded();
    let service = RarityService::build(repo).await.unwrap();
    assert_eq!(service.get_all().len(), 4);
}

#[tokio::test]
async fn get_by_id_and_name() {
    let repo = InMemoryRarityRepository::new_seeded();
    let service = RarityService::build(repo).await.unwrap();

    assert_eq!(
        service
            .get_by_id(RarityId::new(1))
            .map(|r| r.display_name)
            .as_deref(),
        Some("Common")
    );
    assert!(service.get_by_id(RarityId::new(99)).is_none());
}

#[tokio::test]
async fn save_update_delete_refresh_cache() {
    let repo = InMemoryRarityRepository::seed(vec![]);
    let service = RarityService::build(repo).await.unwrap();

    service
        .save(Rarity::new(
            RarityId::new(0),
            "c",
            "Custom",
            "c.png",
            "#fff",
        ))
        .await
        .unwrap();
    assert_eq!(service.get_all().len(), 1);

    let saved = service.get_all()[0].clone();
    service
        .update(Rarity::new(saved.id, "c", "Renamed", "c.png", "#fff"))
        .await
        .unwrap();
    assert_eq!(
        service
            .get_by_id(saved.id)
            .map(|r| r.display_name)
            .as_deref(),
        Some("Renamed")
    );

    service.delete(saved.id).await.unwrap();
    assert!(service.get_all().is_empty());
}
