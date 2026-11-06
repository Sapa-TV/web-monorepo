use super::*;

#[tokio::test]
async fn create_and_get() {
    let repo = InMemoryAdminRepository::new();
    let admin = repo.create("100", Some("tester"), true).await.unwrap();
    assert!(admin.is_root);

    let fetched = repo.get_by_twitch_id("100").await.unwrap().unwrap();
    assert_eq!(fetched.twitch_id, "100");
    assert_eq!(fetched.display_name.as_deref(), Some("tester"));
}

#[tokio::test]
async fn create_conflicts_on_duplicate_id() {
    let repo = InMemoryAdminRepository::new();
    repo.create("100", None, true).await.unwrap();
    let err = repo.create("100", None, false).await.unwrap_err();
    assert!(matches!(err, RepositoryError::Conflict(_)));
}

#[tokio::test]
async fn list_returns_all() {
    let repo = InMemoryAdminRepository::new();
    repo.create("100", None, true).await.unwrap();
    repo.create("200", None, false).await.unwrap();
    assert_eq!(repo.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn set_root_flips_flag() {
    let repo = InMemoryAdminRepository::new();
    repo.create("200", None, false).await.unwrap();
    let admin = repo.set_root("200", true).await.unwrap().unwrap();
    assert!(admin.is_root);
    assert!(!repo.set_root("missing", true).await.unwrap().is_some());
}

#[tokio::test]
async fn update_display_name() {
    let repo = InMemoryAdminRepository::new();
    repo.create("200", Some("old"), false).await.unwrap();
    let admin = repo
        .update_display_name("200", "new")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(admin.display_name.as_deref(), Some("new"));
    assert!(
        repo.update_display_name("missing", "x")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn delete_removes_entry() {
    let repo = InMemoryAdminRepository::new();
    repo.create("200", None, false).await.unwrap();
    assert!(repo.delete_by_twitch_id("200").await.unwrap());
    assert!(!repo.delete_by_twitch_id("200").await.unwrap());
    assert!(repo.get_by_twitch_id("200").await.unwrap().is_none());
}
