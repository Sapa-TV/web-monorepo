use std::sync::Arc;

use crate::db::inmemory_platform::InMemoryPlatformRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::error::UserServiceError;
use crate::platform::PlatformId;

use super::*;

type TestService = UserService<InMemoryUserRepository, InMemoryPlatformRepository>;

async fn test_service() -> TestService {
    UserService::new(
        Arc::new(InMemoryUserRepository::new()),
        Arc::new(InMemoryPlatformRepository::new_seeded()),
    )
}

async fn create_user(svc: &TestService, name: &str) -> User {
    svc.create(name).await.unwrap()
}

#[tokio::test]
async fn ensure_user_by_platform_creates_new() {
    let svc = test_service().await;
    let user_id = svc
        .ensure_user_by_platform("twitch", "999", "New Viewer")
        .await
        .unwrap();
    let user = svc.get_user(user_id).await.unwrap().unwrap();
    assert_eq!(user.display_name, "New Viewer");
    let found = svc
        .find_by_platform("twitch", "999")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, user_id);
}

#[tokio::test]
async fn ensure_user_by_platform_reuses_existing() {
    let svc = test_service().await;
    let first = svc
        .ensure_user_by_platform("twitch", "999", "First")
        .await
        .unwrap();
    let second = svc
        .ensure_user_by_platform("twitch", "999", "First")
        .await
        .unwrap();
    assert_eq!(first, second);
    let user = svc.get_user(first).await.unwrap().unwrap();
    assert_eq!(user.display_name, "First");
}

#[tokio::test]
async fn ensure_user_by_platform_unknown_platform() {
    let svc = test_service().await;
    let err = svc
        .ensure_user_by_platform("unknown", "999", "Viewer")
        .await
        .unwrap_err();
    assert!(matches!(err, UserServiceError::UnknownPlatform(_)));
}

#[tokio::test]
async fn guest_user_id_is_cached() {
    let svc = test_service().await;
    let first = svc.guest_user_id().await.unwrap();
    let second = svc.guest_user_id().await.unwrap();
    assert_eq!(first, second);
    let user = svc.get_user(first).await.unwrap().unwrap();
    assert_eq!(user.display_name, "guest");
}

#[tokio::test]
async fn find_by_platform_unknown_platform() {
    let svc = test_service().await;
    let err = svc.find_by_platform("unknown", "123").await.unwrap_err();
    assert!(matches!(err, UserServiceError::UnknownPlatform(_)));
}

#[tokio::test]
async fn find_by_platform_not_found() {
    let svc = test_service().await;
    let result = svc.find_by_platform("twitch", "123").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn find_by_platform_found() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    svc.link_platform(user.id, "twitch", "123", "tw_user")
        .await
        .unwrap();
    let found = svc
        .find_by_platform("twitch", "123")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, user.id);
}

#[tokio::test]
async fn link_platform_nonexistent_user() {
    let svc = test_service().await;
    let err = svc
        .link_platform(UserId::new(999), "twitch", "123", "u")
        .await
        .unwrap_err();
    assert!(matches!(err, UserServiceError::UserNotFound));
}

#[tokio::test]
async fn link_platform_unknown_platform() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    let err = svc
        .link_platform(user.id, "unknown", "123", "u")
        .await
        .unwrap_err();
    assert!(matches!(err, UserServiceError::UnknownPlatform(_)));
}

#[tokio::test]
async fn update_user_nonexistent() {
    let svc = test_service().await;
    let err = svc.update_user(UserId::new(999), "Name").await.unwrap_err();
    assert!(matches!(err, UserServiceError::UserNotFound));
}

#[tokio::test]
async fn delete_user_nonexistent() {
    let svc = test_service().await;
    let err = svc.delete_user(UserId::new(999)).await.unwrap_err();
    assert!(matches!(err, UserServiceError::UserNotFound));
}

#[tokio::test]
async fn update_platform_username_missing_link() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    let err = svc
        .update_platform_username(user.id, "twitch", "name")
        .await
        .unwrap_err();
    assert!(matches!(err, UserServiceError::PlatformLinkNotFound));
}

#[tokio::test]
async fn delete_platform_missing_link() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    let err = svc.delete_platform(user.id, "twitch").await.unwrap_err();
    assert!(matches!(err, UserServiceError::PlatformLinkNotFound));
}

#[tokio::test]
async fn update_platform_username_unknown_platform() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    let err = svc
        .update_platform_username(user.id, "unknown", "name")
        .await
        .unwrap_err();
    assert!(matches!(err, UserServiceError::UnknownPlatform(_)));
}

#[tokio::test]
async fn update_user_happy_path() {
    let svc = test_service().await;
    let user = create_user(&svc, "Old").await;
    svc.update_user(user.id, "New").await.unwrap();
    let fetched = svc.get_user(user.id).await.unwrap().unwrap();
    assert_eq!(fetched.display_name, "New");
}

#[tokio::test]
async fn delete_user_happy_path() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    svc.delete_user(user.id).await.unwrap();
    assert!(svc.get_user(user.id).await.unwrap().is_none());
}

#[tokio::test]
async fn build_user_resolves_platform_names() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    svc.link_platform(user.id, "twitch", "123", "tw_user")
        .await
        .unwrap();

    let view = svc.build_user(user.id).await.unwrap().unwrap();
    assert_eq!(view.user.display_name, "Viewer");
    assert_eq!(view.platforms.len(), 1);
    assert_eq!(view.platforms[0].platform_name, "twitch");
    assert_eq!(view.platforms[0].platform_user_id, "123");
    assert_eq!(view.platforms[0].platform_username, "tw_user");
}

#[tokio::test]
async fn build_user_nonexistent() {
    let svc = test_service().await;
    assert!(svc.build_user(UserId::new(999)).await.unwrap().is_none());
}

#[tokio::test]
async fn build_user_unknown_platform_falls_back_to_empty_name() {
    let svc = test_service().await;
    let user = create_user(&svc, "Viewer").await;
    svc.link_platform(user.id, "twitch", "123", "tw_user")
        .await
        .unwrap();
    let user_platforms = svc.get_platforms(user.id).await.unwrap();
    let mut broken = user_platforms;
    broken[0].platform_id = PlatformId::new(999);

    let resolved = svc.resolve_user_platforms(broken).await.unwrap();
    assert_eq!(resolved[0].platform_name, "");
}
