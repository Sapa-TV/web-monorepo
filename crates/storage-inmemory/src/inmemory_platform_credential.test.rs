use super::*;

#[tokio::test]
async fn roundtrip() {
    let repo = InMemoryPlatformCredentialRepository::new();
    assert_eq!(
        repo.load_credential(PlatformId::TWITCH).await.unwrap(),
        None
    );

    repo.save_credential(PlatformId::TWITCH, "refresh_token_1")
        .await
        .unwrap();
    assert_eq!(
        repo.load_credential(PlatformId::TWITCH)
            .await
            .unwrap()
            .as_deref(),
        Some("refresh_token_1")
    );
}

#[tokio::test]
async fn clear_removes_credential() {
    let repo = InMemoryPlatformCredentialRepository::new();
    repo.save_credential(PlatformId::VK_VIDEO_LIVE, "token")
        .await
        .unwrap();
    repo.clear_credential(PlatformId::VK_VIDEO_LIVE)
        .await
        .unwrap();
    assert_eq!(
        repo.load_credential(PlatformId::VK_VIDEO_LIVE)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn credentials_are_per_platform() {
    let repo = InMemoryPlatformCredentialRepository::new();
    repo.save_credential(PlatformId::TWITCH, "twitch_token")
        .await
        .unwrap();
    assert_eq!(
        repo.load_credential(PlatformId::TWITCH)
            .await
            .unwrap()
            .as_deref(),
        Some("twitch_token")
    );
    assert_eq!(
        repo.load_credential(PlatformId::YOUTUBE).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn save_trims_credential() {
    let repo = InMemoryPlatformCredentialRepository::new();
    repo.save_credential(PlatformId::TWITCH, "  tok  ")
        .await
        .unwrap();
    assert_eq!(
        repo.load_credential(PlatformId::TWITCH)
            .await
            .unwrap()
            .as_deref(),
        Some("tok")
    );
}
