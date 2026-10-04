use std::time::Duration;

use tokio::time::sleep;

use crate::db::sqlite::test_pool;

use super::*;

async fn repo() -> SqliteActionRepository {
    let (pool, _path) = test_pool().await;
    SqliteActionRepository::new(pool)
}

#[tokio::test]
async fn create_and_get_roundtrip_all_kinds() {
    let repo = repo().await;

    for (name, kind) in [
        ("noop", ActionKind::NoAction),
        ("spin", ActionKind::EnqueueRoulette),
        (
            "reply",
            ActionKind::ChatReply {
                message_template: "hi {username}".to_string(),
            },
        ),
    ] {
        let saved = repo.create(name, kind.clone(), true).await.unwrap();
        let fetched = repo.get_by_id(saved.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, name);
        assert_eq!(fetched.kind, kind);
        assert_eq!(fetched.created_at, fetched.updated_at);
    }

    assert!(repo.get_by_id(ActionId::new(999)).await.unwrap().is_none());
}

#[tokio::test]
async fn list_returns_in_insertion_order() {
    let repo = repo().await;
    assert!(repo.list().await.unwrap().is_empty());

    repo.create("a", ActionKind::NoAction, true).await.unwrap();
    repo.create("b", ActionKind::EnqueueRoulette, false)
        .await
        .unwrap();

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].name, "a");
    assert_eq!(all[1].name, "b");
    assert!(!all[1].enabled);
}

#[tokio::test]
async fn update_preserves_created_at_and_missing_is_none() {
    let repo = repo().await;
    let saved = repo
        .create("old", ActionKind::NoAction, true)
        .await
        .unwrap();
    sleep(Duration::from_millis(5)).await;

    let mut next = saved.clone();
    next.name = "new".to_string();
    next.kind = ActionKind::ChatReply {
        message_template: "{text}".to_string(),
    };
    next.enabled = false;
    let updated = repo.update(next).await.unwrap().unwrap();

    assert_eq!(updated.name, "new");
    assert_eq!(updated.created_at, saved.created_at);
    assert!(updated.updated_at > saved.updated_at);

    let mut missing = saved.clone();
    missing.id = ActionId::new(999);
    assert!(repo.update(missing).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_reports_presence() {
    let repo = repo().await;
    let saved = repo.create("x", ActionKind::NoAction, true).await.unwrap();

    assert!(repo.delete(saved.id).await.unwrap());
    assert!(!repo.delete(saved.id).await.unwrap());
}
