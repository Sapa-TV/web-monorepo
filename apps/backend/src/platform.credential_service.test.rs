use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;

use super::*;

fn test_service() -> PlatformCredentialService<InMemoryPlatformCredentialRepository> {
    PlatformCredentialService::new(Arc::new(InMemoryPlatformCredentialRepository::new()))
}

#[tokio::test]
async fn initial_revision_is_zero() {
    let service = test_service();
    assert_eq!(*service.subscribe_lifecycle().borrow(), 0);
}

#[tokio::test]
async fn save_credential_persists_and_bumps() {
    let service = test_service();
    let mut rx = service.subscribe_lifecycle();

    service
        .save_credential(PlatformId::TWITCH, "token")
        .await
        .unwrap();

    assert_eq!(
        service
            .load_credential(PlatformId::TWITCH)
            .await
            .unwrap()
            .as_deref(),
        Some("token")
    );
    assert!(rx.has_changed().unwrap());
    rx.changed().await.unwrap();
    assert_eq!(rx.borrow_and_update().clone(), 1);
}

#[tokio::test]
async fn clear_credential_bumps() {
    let service = test_service();
    service
        .save_credential(PlatformId::TWITCH, "token")
        .await
        .unwrap();
    let mut rx = service.subscribe_lifecycle();

    service.clear_credential(PlatformId::TWITCH).await.unwrap();

    assert_eq!(
        service.load_credential(PlatformId::TWITCH).await.unwrap(),
        None
    );
    assert!(rx.has_changed().unwrap());
    rx.changed().await.unwrap();
    assert_eq!(rx.borrow_and_update().clone(), 2);
}

#[tokio::test]
async fn save_rotated_does_not_bump() {
    let service = test_service();
    let rx = service.subscribe_lifecycle();

    service
        .save_rotated(PlatformId::TWITCH, "rotated")
        .await
        .unwrap();

    assert_eq!(
        service
            .load_credential(PlatformId::TWITCH)
            .await
            .unwrap()
            .as_deref(),
        Some("rotated")
    );
    assert_eq!(*rx.borrow(), 0);
    assert!(
        !rx.has_changed().unwrap(),
        "rotation must not notify lifecycle"
    );
}

#[tokio::test]
async fn lifecycle_is_revision_not_payload() {
    let service = test_service();
    let mut rx = service.subscribe_lifecycle();
    service
        .save_credential(PlatformId::TWITCH, "first")
        .await
        .unwrap();
    service
        .save_credential(PlatformId::TWITCH, "second")
        .await
        .unwrap();
    service
        .save_credential(PlatformId::YOUTUBE, "yt")
        .await
        .unwrap();

    rx.changed().await.unwrap();
    assert_eq!(rx.borrow_and_update().clone(), 3);
    assert_eq!(service.revision(), 3);
}

#[tokio::test]
async fn revision_is_monotonic_under_concurrency() {
    let service = Arc::new(test_service());
    let mut handles = Vec::new();
    for i in 0..10_u64 {
        let service = Arc::clone(&service);
        handles.push(tokio::spawn(async move {
            service
                .save_credential(PlatformId::TWITCH, &format!("tok-{i}"))
                .await
                .unwrap();
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
    assert_eq!(service.revision(), 10, "no revision lost under concurrency");
    assert!(
        service
            .load_credential(PlatformId::TWITCH)
            .await
            .unwrap()
            .is_some()
    );
}
