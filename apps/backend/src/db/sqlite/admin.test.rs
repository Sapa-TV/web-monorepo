use crate::db::sqlite::test_pool;

use super::*;

async fn repo() -> SqliteAdminRepository {
    let (pool, _path) = test_pool().await;
    SqliteAdminRepository::new(pool)
}

#[tokio::test]
async fn create_and_get() {
    let repo = repo().await;
    let admin = repo.create("100", Some("tester"), true).await.unwrap();
    assert!(admin.is_root);

    let fetched = repo.get_by_twitch_id("100").await.unwrap().unwrap();
    assert_eq!(fetched.twitch_id, "100");
    assert_eq!(fetched.display_name.as_deref(), Some("tester"));
}

#[tokio::test]
async fn create_conflict_on_duplicate() {
    let repo = repo().await;
    repo.create("100", None, false).await.unwrap();

    let err = repo.create("100", None, true).await.unwrap_err();
    assert!(matches!(
        err,
        RepositoryError::Conflict(ref m) if m.contains("already exists")
    ));
}

#[tokio::test]
async fn update_display_name_missing_is_none() {
    let repo = repo().await;
    let updated = repo.update_display_name("404", "nobody").await.unwrap();
    assert!(updated.is_none());
}

#[tokio::test]
async fn update_display_name_and_list_order() {
    let repo = repo().await;
    repo.create("2", None, false).await.unwrap();
    repo.create("1", None, false).await.unwrap();

    let updated = repo
        .update_display_name("2", "second")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.display_name.as_deref(), Some("second"));

    let listed = repo.list().await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].twitch_id, "2");
    assert_eq!(listed[1].twitch_id, "1");
}

#[tokio::test]
async fn set_root_updates_flag() {
    let repo = repo().await;
    repo.create("7", None, false).await.unwrap();

    let promoted = repo.set_root("7", true).await.unwrap().unwrap();
    assert!(promoted.is_root);
    assert!(!repo.set_root("404", true).await.unwrap().is_some());
}

#[tokio::test]
async fn delete_reports_presence() {
    let repo = repo().await;
    repo.create("9", None, false).await.unwrap();

    assert!(repo.delete_by_twitch_id("9").await.unwrap());
    assert!(!repo.delete_by_twitch_id("9").await.unwrap());
}
