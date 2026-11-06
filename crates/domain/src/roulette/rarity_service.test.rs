use storage_inmemory::inmemory_rarity::InMemoryRarityRepository;

use super::*;

fn rarity(id: u32, name: &str, display_name: &str) -> Rarity {
    Rarity::new(
        RarityId::new(id),
        name,
        display_name,
        "rarity.png",
        "#000000",
    )
}

#[tokio::test]
async fn build_loads_repository_rows() {
    let repo = InMemoryRarityRepository::seed(vec![
        rarity(0, "common", "Common"),
        rarity(0, "rare", "Rare"),
    ]);
    let service = RarityService::build(repo).await.unwrap();

    assert_eq!(service.get_all().len(), 2);
}

#[tokio::test]
async fn get_by_id_and_missing() {
    let repo = InMemoryRarityRepository::seed(vec![rarity(0, "common", "Common")]);
    let service = RarityService::build(repo).await.unwrap();
    let saved = service.get_all()[0].clone();

    assert_eq!(
        service.get_by_id(saved.id).map(|r| r.display_name),
        Some("Common".to_string())
    );
    let missing = RarityId::new(saved.id.get() + 10_000);
    assert!(service.get_by_id(missing).is_none());
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
