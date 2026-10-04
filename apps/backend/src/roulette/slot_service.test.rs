use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;

const COMMON: RarityId = RarityId::new(1);

use super::*;

#[tokio::test]
async fn test_total_weight_calc() {
    let slots = vec![
        RouletteSlot::new(RouletteSlotId::new(0), "Test_1", COMMON, 123, "Action 1"),
        RouletteSlot::new(RouletteSlotId::new(0), "Test_2", COMMON, 246, "Action 2"),
    ];
    let repo = InMemoryRouletteSlotRepository::seed(slots);
    let slot_service = RouletteSlotService::build(repo).await.unwrap();

    let total_weight = slot_service.total_weight();
    assert_eq!(total_weight, 369);
}

#[tokio::test]
async fn test_mid_boundary_switching() {
    let slots = vec![
        RouletteSlot::new(RouletteSlotId::new(0), "Test_1", COMMON, 10, "Action 1"),
        RouletteSlot::new(RouletteSlotId::new(0), "Test_2", COMMON, 20, "Action 2"),
    ];
    let repo = InMemoryRouletteSlotRepository::seed(slots);
    let slot_service = RouletteSlotService::build(repo).await.unwrap();

    let slot = slot_service.get_slot_by_weight(0).unwrap();
    assert_eq!(slot.name, "Test_1".to_string());

    let slot = slot_service.get_slot_by_weight(9).unwrap();
    assert_eq!(slot.name, "Test_1".to_string());

    let slot = slot_service.get_slot_by_weight(10).unwrap();
    assert_eq!(slot.name, "Test_2".to_string());
}

#[tokio::test]
async fn test_absolute_favorite() {
    let slots = vec![
        RouletteSlot::new(RouletteSlotId::new(0), "Loser_1", COMMON, 0, "Loser 1"),
        RouletteSlot::new(RouletteSlotId::new(0), "Winner", COMMON, 20, "Winner"),
        RouletteSlot::new(RouletteSlotId::new(0), "Loser_2", COMMON, 0, "Loser 1"),
    ];
    let repo = InMemoryRouletteSlotRepository::seed(slots);
    let slot_service = RouletteSlotService::build(repo).await.unwrap();

    let slot = slot_service.get_slot_by_weight(0).unwrap();
    assert_eq!(slot.name, "Winner".to_string());

    let slot = slot_service.get_slot_by_weight(10).unwrap();
    assert_eq!(slot.name, "Winner".to_string());

    let slot = slot_service.get_slot_by_weight(19).unwrap();
    assert_eq!(slot.name, "Winner".to_string());
}

#[tokio::test]
async fn test_fallback_heaviest() {
    let slots = vec![
        RouletteSlot::new(RouletteSlotId::new(0), "Loser_1", COMMON, 0, "Loser 1"),
        RouletteSlot::new(RouletteSlotId::new(0), "Winner", COMMON, 20, "Winner"),
        RouletteSlot::new(RouletteSlotId::new(0), "Loser_2", COMMON, 0, "Loser 1"),
    ];
    let repo = InMemoryRouletteSlotRepository::seed(slots);
    let slot_service = RouletteSlotService::build(repo).await.unwrap();

    let slot = slot_service.get_slot_by_weight(30).unwrap();
    assert_eq!(slot.name, "Winner".to_string());
}

#[tokio::test]
async fn test_add_and_get_slots() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let slot_service = RouletteSlotService::build(repo).await.unwrap();

    slot_service
        .add_slot(RouletteSlot::new(
            RouletteSlotId::new(0),
            "New",
            COMMON,
            50,
            "Action",
        ))
        .await
        .unwrap();
    assert_eq!(slot_service.get_slots().len(), 1);
    assert_eq!(slot_service.total_weight(), 50);
}

#[tokio::test]
async fn test_delete_slot() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let slot_service = RouletteSlotService::build(repo).await.unwrap();

    slot_service
        .add_slot(RouletteSlot::new(
            RouletteSlotId::new(0),
            "ToDelete",
            COMMON,
            10,
            "Act",
        ))
        .await
        .unwrap();
    let id = slot_service.get_slots()[0].id;
    slot_service.delete_slot(id).await.unwrap();
    assert!(slot_service.get_slots().is_empty());
}

#[tokio::test]
async fn test_edit_slot() {
    let repo = InMemoryRouletteSlotRepository::seed(vec![]);
    let slot_service = RouletteSlotService::build(repo).await.unwrap();

    slot_service
        .add_slot(RouletteSlot::new(
            RouletteSlotId::new(0),
            "Original",
            COMMON,
            10,
            "Act",
        ))
        .await
        .unwrap();

    let id = slot_service.get_slots()[0].id;
    slot_service
        .edit_slot(RouletteSlot::new(id, "Edited", COMMON, 99, "NewAct"))
        .await
        .unwrap();

    assert_eq!(slot_service.get_slots()[0].name, "Edited");
    assert_eq!(slot_service.get_slots()[0].weight, 99);
}
