use super::*;

#[tokio::test]
async fn find_by_name_found() {
    let repo = InMemoryPlatformRepository::new_seeded();
    let result = repo.find_by_name("twitch").await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "twitch");
}

#[tokio::test]
async fn find_by_name_not_found() {
    let repo = InMemoryPlatformRepository::new_seeded();
    let result = repo.find_by_name("unknown").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn load_all_returns_seeded() {
    let repo = InMemoryPlatformRepository::new_seeded();
    let all = repo.load_all().await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].name, "twitch");
    assert_eq!(all[1].name, "youtube");
    assert_eq!(all[2].name, "vk_video_live");
}
