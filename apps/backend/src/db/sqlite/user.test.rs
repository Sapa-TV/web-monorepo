use std::time::Duration;

use chrono::Utc;
use tokio::time::sleep;

use crate::db::sqlite::test_pool;
use crate::platform::PlatformId;

use super::*;

const TWITCH: PlatformId = PlatformId::TWITCH;
const YOUTUBE: PlatformId = PlatformId::YOUTUBE;

async fn repo() -> SqliteUserRepository {
    let (pool, _path) = test_pool().await;
    SqliteUserRepository::new(pool)
}

#[tokio::test]
async fn create_assigns_sequential_ids() {
    let repo = repo().await;
    let first = repo.create("Viewer").await.unwrap();
    let second = repo.create("Another").await.unwrap();

    assert_eq!(first.id.get(), 1);
    assert_eq!(second.id.get(), 2);
    assert_eq!(first.created_at, first.updated_at);
    assert!(repo.get_by_id(first.id).await.unwrap().is_some());
    assert!(repo.get_by_id(UserId::new(999)).await.unwrap().is_none());
}

#[tokio::test]
async fn find_by_platform_joins_linked_user() {
    let repo = repo().await;
    let user = repo.create("Viewer").await.unwrap();

    assert!(
        repo.find_by_platform(TWITCH, "t-1")
            .await
            .unwrap()
            .is_none()
    );

    repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
        .await
        .unwrap();

    let found = repo.find_by_platform(TWITCH, "t-1").await.unwrap().unwrap();
    assert_eq!(found.id, user.id);
    assert_eq!(found.display_name, "Viewer");

    assert!(
        repo.find_by_platform(YOUTUBE, "t-1")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn link_platform_conflict_on_duplicate_across_users() {
    let repo = repo().await;
    let first = repo.create("A").await.unwrap();
    let second = repo.create("B").await.unwrap();

    repo.link_platform(first.id, TWITCH, "same", "a")
        .await
        .unwrap();

    let err = repo
        .link_platform(second.id, TWITCH, "same", "b")
        .await
        .unwrap_err();
    match err {
        RepositoryError::Conflict(m) => {
            assert_eq!(m, "platform_user_id already linked to another user")
        }
        other => panic!("expected Conflict, got {other:?}"),
    }
}

#[tokio::test]
async fn link_platform_bumps_user_updated_at() {
    let repo = repo().await;
    let user = repo.create("Viewer").await.unwrap();
    sleep(Duration::from_millis(5)).await;

    repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
        .await
        .unwrap();

    let after = repo.get_by_id(user.id).await.unwrap().unwrap();
    assert!(after.updated_at > user.updated_at);
}

#[tokio::test]
async fn get_platforms_lists_links_in_order() {
    let repo = repo().await;
    let user = repo.create("Viewer").await.unwrap();

    assert!(repo.get_platforms(user.id).await.unwrap().is_empty());

    repo.link_platform(user.id, YOUTUBE, "y-1", "viewer_yt")
        .await
        .unwrap();
    repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
        .await
        .unwrap();

    let platforms = repo.get_platforms(user.id).await.unwrap();
    assert_eq!(platforms.len(), 2);
    assert_eq!(platforms[0].platform_id, YOUTUBE);
    assert_eq!(platforms[0].platform_user_id, "y-1");
    assert_eq!(platforms[1].platform_id, TWITCH);
}

#[tokio::test]
async fn update_display_name_bumps_and_missing_is_none() {
    let repo = repo().await;
    let user = repo.create("Old").await.unwrap();
    sleep(Duration::from_millis(5)).await;

    let updated = repo
        .update_display_name(user.id, "New")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.display_name, "New");
    assert!(updated.updated_at > user.updated_at);

    assert!(
        repo.update_display_name(UserId::new(999), "X")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn update_platform_username_updates_link_and_bumps_user() {
    let repo = repo().await;
    let user = repo.create("Viewer").await.unwrap();

    assert!(
        repo.update_platform_username(user.id, TWITCH, "new_nick")
            .await
            .unwrap()
            .is_none()
    );

    repo.link_platform(user.id, TWITCH, "t-1", "old_nick")
        .await
        .unwrap();
    sleep(Duration::from_millis(5)).await;

    let link = repo
        .update_platform_username(user.id, TWITCH, "new_nick")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(link.platform_username, "new_nick");
    assert_eq!(link.platform_user_id, "t-1");

    let platforms = repo.get_platforms(user.id).await.unwrap();
    assert_eq!(platforms[0].platform_username, "new_nick");
    let after = repo.get_by_id(user.id).await.unwrap().unwrap();
    assert!(after.updated_at > Utc::now() - chrono::Duration::seconds(1));
}

#[tokio::test]
async fn delete_platform_reports_presence_and_bumps_user() {
    let repo = repo().await;
    let user = repo.create("Viewer").await.unwrap();

    assert!(!repo.delete_platform(user.id, TWITCH).await.unwrap());

    repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
        .await
        .unwrap();
    sleep(Duration::from_millis(5)).await;

    assert!(repo.delete_platform(user.id, TWITCH).await.unwrap());
    assert!(repo.get_platforms(user.id).await.unwrap().is_empty());
    let after = repo.get_by_id(user.id).await.unwrap().unwrap();
    assert!(after.updated_at > user.updated_at);
}

#[tokio::test]
async fn delete_user_cascades_links() {
    let repo = repo().await;
    let user = repo.create("Viewer").await.unwrap();
    repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
        .await
        .unwrap();

    assert!(repo.delete_user(user.id).await.unwrap());
    assert!(!repo.delete_user(user.id).await.unwrap());
    assert!(
        repo.find_by_platform(TWITCH, "t-1")
            .await
            .unwrap()
            .is_none()
    );
    assert!(repo.get_platforms(user.id).await.unwrap().is_empty());
}
