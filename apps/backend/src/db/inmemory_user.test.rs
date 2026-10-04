use std::time::Duration;
use tokio::time::sleep;

use crate::platform::PlatformId;

use super::*;

const TWITCH: PlatformId = PlatformId::new(1);
const YOUTUBE: PlatformId = PlatformId::new(2);

#[tokio::test]
async fn create_user() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    assert_eq!(user.display_name, "Viewer");
    assert_eq!(user.id, UserId::new(1));
}

#[tokio::test]
async fn find_by_platform_found() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    repo.link_platform(user.id, TWITCH, "123", "viewer_name")
        .await
        .unwrap();

    let found = repo.find_by_platform(TWITCH, "123").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, user.id);
}

#[tokio::test]
async fn find_by_platform_not_found() {
    let repo = InMemoryUserRepository::new();
    let result = repo.find_by_platform(TWITCH, "unknown").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn link_platform_duplicate_global() {
    let repo = InMemoryUserRepository::new();
    let user1 = repo.create("A").await.unwrap();
    let user2 = repo.create("B").await.unwrap();
    repo.link_platform(user1.id, TWITCH, "123", "user_a")
        .await
        .unwrap();

    let err = repo
        .link_platform(user2.id, TWITCH, "123", "user_b")
        .await
        .unwrap_err();
    assert_eq!(
        err,
        RepositoryError::Conflict("platform_user_id already linked to another user".to_string())
    );
}

#[tokio::test]
async fn get_platforms_returns_linked() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    repo.link_platform(user.id, TWITCH, "t123", "twitch_user")
        .await
        .unwrap();
    repo.link_platform(user.id, YOUTUBE, "y456", "yt_user")
        .await
        .unwrap();

    let platforms = repo.get_platforms(user.id).await.unwrap();
    assert_eq!(platforms.len(), 2);
}

#[tokio::test]
async fn get_platforms_empty() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    let platforms = repo.get_platforms(user.id).await.unwrap();
    assert!(platforms.is_empty());
}

#[tokio::test]
async fn update_display_name() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("OldName").await.unwrap();
    let updated = repo
        .update_display_name(user.id, "NewName")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.display_name, "NewName");

    let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
    assert_eq!(fetched.display_name, "NewName");
}

#[tokio::test]
async fn update_display_name_nonexistent() {
    let repo = InMemoryUserRepository::new();
    let result = repo
        .update_display_name(UserId::new(999), "Name")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn update_platform_username() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    repo.link_platform(user.id, TWITCH, "123", "old_name")
        .await
        .unwrap();

    let updated = repo
        .update_platform_username(user.id, TWITCH, "new_name")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.platform_username, "new_name");

    let platforms = repo.get_platforms(user.id).await.unwrap();
    assert_eq!(platforms[0].platform_username, "new_name");
}

#[tokio::test]
async fn update_platform_username_nonexistent_link() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    let result = repo
        .update_platform_username(user.id, TWITCH, "name")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn get_by_id_found() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
    assert_eq!(fetched.id, user.id);
    assert_eq!(fetched.display_name, "Viewer");
}

#[tokio::test]
async fn get_by_id_not_found() {
    let repo = InMemoryUserRepository::new();
    let result = repo.get_by_id(UserId::new(999)).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn delete_platform_removes_link() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    repo.link_platform(user.id, TWITCH, "123", "user")
        .await
        .unwrap();
    repo.link_platform(user.id, YOUTUBE, "456", "yt_user")
        .await
        .unwrap();

    repo.delete_platform(user.id, TWITCH).await.unwrap();
    let platforms = repo.get_platforms(user.id).await.unwrap();
    assert_eq!(platforms.len(), 1);
    assert_eq!(platforms[0].platform_id, YOUTUBE);
}

#[tokio::test]
async fn delete_platform_nonexistent() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    let result = repo.delete_platform(user.id, TWITCH).await.unwrap();
    assert!(!result);
}

#[tokio::test]
async fn delete_user_removes_user_and_platforms() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    repo.link_platform(user.id, TWITCH, "123", "user")
        .await
        .unwrap();

    let deleted = repo.delete_user(user.id).await.unwrap();
    assert!(deleted);

    let result = repo.get_by_id(user.id).await.unwrap();
    assert!(result.is_none());

    let platforms = repo.get_platforms(user.id).await.unwrap();
    assert!(platforms.is_empty());
}

#[tokio::test]
async fn delete_user_nonexistent() {
    let repo = InMemoryUserRepository::new();
    let result = repo.delete_user(UserId::new(999)).await.unwrap();
    assert!(!result);
}

#[tokio::test]
async fn updated_at_changes_on_update() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    let original = user.updated_at;

    let updated = repo
        .update_display_name(user.id, "New")
        .await
        .unwrap()
        .unwrap();
    assert!(updated.updated_at > original);
}

#[tokio::test]
async fn updated_at_changes_on_link_platform() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    let original = user.updated_at;

    repo.link_platform(user.id, TWITCH, "123", "user")
        .await
        .unwrap();

    let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
    assert!(fetched.updated_at > original);
}

#[tokio::test]
async fn updated_at_changes_on_delete_platform() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("Viewer").await.unwrap();
    repo.link_platform(user.id, TWITCH, "123", "user")
        .await
        .unwrap();
    let before_delete = repo.get_by_id(user.id).await.unwrap().unwrap().updated_at;

    sleep(Duration::from_millis(10)).await;
    repo.delete_platform(user.id, TWITCH).await.unwrap();

    let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
    assert!(fetched.updated_at > before_delete);
}
