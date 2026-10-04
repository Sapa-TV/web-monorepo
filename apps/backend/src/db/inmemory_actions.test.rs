use super::*;

fn reply_kind() -> ActionKind {
    ActionKind::ChatReply {
        message_template: "hi {username}".to_string(),
    }
}

#[tokio::test]
async fn create_and_get() {
    let repo = InMemoryActionRepository::new();
    let action = repo.create("reply", reply_kind(), true).await.unwrap();
    assert_eq!(action.id, ActionId::new(1));

    let fetched = repo.get_by_id(ActionId::new(1)).await.unwrap().unwrap();
    assert_eq!(fetched.name, "reply");
    assert_eq!(fetched.kind, reply_kind());
}

#[tokio::test]
async fn list_returns_all() {
    let repo = InMemoryActionRepository::new();
    repo.create("a", reply_kind(), true).await.unwrap();
    repo.create("b", ActionKind::EnqueueRoulette, false)
        .await
        .unwrap();
    assert_eq!(repo.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn update_replaces_fields_and_touches_updated_at() {
    let repo = InMemoryActionRepository::new();
    repo.create("a", reply_kind(), true).await.unwrap();
    let original = repo.get_by_id(ActionId::new(1)).await.unwrap().unwrap();
    let updated = repo
        .update(Action::new(
            original.id,
            "renamed".to_string(),
            ActionKind::EnqueueRoulette,
            original.enabled,
            original.created_at,
            original.updated_at,
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "renamed");
    assert_eq!(updated.kind, ActionKind::EnqueueRoulette);
    assert_eq!(updated.created_at, original.created_at);
    assert!(updated.updated_at > original.updated_at);

    let ghost = Action::new(
        ActionId::new(99),
        original.name.clone(),
        original.kind.clone(),
        original.enabled,
        original.created_at,
        original.updated_at,
    );
    assert!(repo.update(ghost).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_removes_entry() {
    let repo = InMemoryActionRepository::new();
    repo.create("a", reply_kind(), true).await.unwrap();
    assert!(repo.delete(ActionId::new(1)).await.unwrap());
    assert!(!repo.delete(ActionId::new(1)).await.unwrap());
    assert!(repo.get_by_id(ActionId::new(1)).await.unwrap().is_none());
}
