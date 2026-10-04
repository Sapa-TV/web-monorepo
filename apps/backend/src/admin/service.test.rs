use std::sync::Arc;

use crate::db::inmemory_admin::InMemoryAdminRepository;

use super::*;

type TestService = AdminService<InMemoryAdminRepository>;

fn test_service() -> TestService {
    AdminService::new(Arc::new(InMemoryAdminRepository::new()))
}

#[tokio::test]
async fn seed_creates_root_admin() {
    let svc = test_service();
    svc.seed("100").await.unwrap();

    let admin = svc.get("100").await.unwrap().unwrap();
    assert!(admin.is_root);
    assert!(svc.is_admin("100").await.unwrap());
    assert!(svc.is_root("100").await.unwrap());
}

#[tokio::test]
async fn seed_promotes_existing_non_root() {
    let svc = test_service();
    svc.add("100", Some("tester")).await.unwrap();
    assert!(!svc.is_root("100").await.unwrap());

    svc.seed("100").await.unwrap();
    assert!(svc.is_root("100").await.unwrap());
}

#[tokio::test]
async fn seed_is_idempotent() {
    let svc = test_service();
    svc.seed("100").await.unwrap();
    svc.seed("100").await.unwrap();
    assert_eq!(svc.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn seed_non_root_creates_regular_admin() {
    let svc = test_service();
    svc.seed_non_root("200").await.unwrap();

    let admin = svc.get("200").await.unwrap().unwrap();
    assert!(!admin.is_root);
}

#[tokio::test]
async fn seed_non_root_is_idempotent() {
    let svc = test_service();
    svc.seed_non_root("200").await.unwrap();
    svc.seed_non_root("200").await.unwrap();
    assert_eq!(svc.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn seed_non_root_keeps_existing_root() {
    let svc = test_service();
    svc.seed("100").await.unwrap();
    svc.seed_non_root("100").await.unwrap();
    assert!(svc.is_root("100").await.unwrap());
}

#[tokio::test]
async fn add_creates_non_root_admin() {
    let svc = test_service();
    let admin = svc.add("200", Some("moderator")).await.unwrap();
    assert!(!admin.is_root);
    assert!(svc.is_admin("200").await.unwrap());
    assert!(!svc.is_root("200").await.unwrap());
}

#[tokio::test]
async fn add_duplicate_is_rejected() {
    let svc = test_service();
    svc.add("200", None).await.unwrap();
    let err = svc.add("200", None).await.unwrap_err();
    assert!(matches!(err, AdminServiceError::AlreadyAdmin));
}

#[tokio::test]
async fn remove_deletes_admin() {
    let svc = test_service();
    svc.add("200", None).await.unwrap();
    svc.remove("200").await.unwrap();
    assert!(!svc.is_admin("200").await.unwrap());
}

#[tokio::test]
async fn remove_missing_is_not_found() {
    let svc = test_service();
    let err = svc.remove("missing").await.unwrap_err();
    assert!(matches!(err, AdminServiceError::AdminNotFound));
}

#[tokio::test]
async fn cannot_remove_last_root() {
    let svc = test_service();
    svc.seed("100").await.unwrap();
    let err = svc.remove("100").await.unwrap_err();
    assert!(matches!(err, AdminServiceError::CannotRemoveLastRoot));
}

#[tokio::test]
async fn can_remove_first_root_when_second_exists() {
    let svc = test_service();
    svc.seed("100").await.unwrap();
    svc.add("200", None).await.unwrap();
    svc.set_root("200", true).await.unwrap();
    svc.remove("100").await.unwrap();
    assert!(svc.is_admin("200").await.unwrap());
}

#[tokio::test]
async fn record_login_updates_display_name() {
    let svc = test_service();
    svc.add("200", Some("old_name")).await.unwrap();
    svc.update_display_name("200", "new_name").await.unwrap();
    let admin = svc.get("200").await.unwrap().unwrap();
    assert_eq!(admin.display_name.as_deref(), Some("new_name"));
}
