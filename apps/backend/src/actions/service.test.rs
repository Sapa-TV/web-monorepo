use crate::db::inmemory_actions::InMemoryActionRepository;

use super::*;

fn test_service() -> ActionService<InMemoryActionRepository> {
    ActionService::new(Arc::new(InMemoryActionRepository::new()))
}

#[tokio::test]
async fn create_bumps_lifecycle() {
    let service = test_service();
    let mut rx = service.subscribe_lifecycle();
    service
        .create("reply", ActionKind::EnqueueRoulette, true)
        .await
        .unwrap();
    rx.changed().await.unwrap();
    assert_eq!(rx.borrow_and_update().clone(), 1);
}

#[tokio::test]
async fn get_missing_returns_none() {
    let service = test_service();
    assert!(service.get(ActionId::new(999)).await.unwrap().is_none());
}

#[tokio::test]
async fn update_missing_is_not_found() {
    let service = test_service();
    let err = service
        .update(Action::new(
            ActionId::new(999),
            "x".to_string(),
            ActionKind::EnqueueRoulette,
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        ))
        .await
        .unwrap_err();
    assert!(matches!(err, ActionServiceError::ActionNotFound));
}

#[tokio::test]
async fn delete_missing_is_not_found() {
    let service = test_service();
    let err = service.delete(ActionId::new(999)).await.unwrap_err();
    assert!(matches!(err, ActionServiceError::ActionNotFound));
}

#[tokio::test]
async fn delete_removes_and_bumps() {
    let service = test_service();
    let action = service
        .create("reply", ActionKind::EnqueueRoulette, true)
        .await
        .unwrap();
    service.delete(action.id).await.unwrap();
    assert!(service.get(action.id).await.unwrap().is_none());
}
