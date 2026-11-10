use std::sync::Arc;
use std::time::Duration;

use crate::admin::service::AdminService;
use model::config::runtime::RuntimeConfig;
use model::user::UserId;
use storage_inmemory::inmemory_admin::InMemoryAdminRepository;
use storage_inmemory::inmemory_session::InMemorySessionRepository;

use super::*;

type TestService = SessionService<InMemorySessionRepository, InMemoryAdminRepository>;

fn test_settings(ttl_secs: u64) -> SharedSettings {
    let mut config = RuntimeConfig::test_runtime("test-key");
    config.session.ttl_secs = ttl_secs;
    SharedSettings::test_new(config)
}

fn test_service() -> TestService {
    SessionService::new(
        Arc::new(InMemorySessionRepository::new()),
        Arc::new(AdminService::new(Arc::new(InMemoryAdminRepository::new()))),
        test_settings(60 * 60),
    )
}

#[tokio::test]
async fn ticket_is_one_time_use() {
    let svc = test_service();
    let ticket = svc
        .create_login_ticket(PlatformId::TWITCH, "123", Some("tester"))
        .await
        .unwrap();

    let consumed = svc
        .consume_login_ticket(ticket.ticket.as_str())
        .await
        .unwrap();
    assert_eq!(consumed.platform_user_id, "123");
    assert_eq!(consumed.platform_username.as_deref(), Some("tester"));

    let err = svc
        .consume_login_ticket(ticket.ticket.as_str())
        .await
        .unwrap_err();
    assert!(matches!(err, SessionServiceError::InvalidTicket));
}

#[tokio::test]
async fn unknown_ticket_is_rejected() {
    let svc = test_service();
    let err = svc.consume_login_ticket("bogus").await.unwrap_err();
    assert!(matches!(err, SessionServiceError::InvalidTicket));
}

#[tokio::test]
async fn issue_session_generates_token_and_ttl() {
    let svc = test_service();
    let session = svc
        .issue_session_for(
            UserId::new(7),
            Some(PlatformId::TWITCH),
            Some("123"),
            None,
            Duration::from_secs(60 * 60),
        )
        .await
        .unwrap();
    assert_eq!(session.user_id, UserId::new(7));
    assert_eq!(session.platform_user_id.as_deref(), Some("123"));
    assert_eq!(session.token.as_str().len(), 64);

    let validated = svc.validate_session(session.token.as_str()).await.unwrap();
    assert_eq!(validated.user_id, UserId::new(7));
    assert_eq!(validated.platform_user_id.as_deref(), Some("123"));
}

#[tokio::test]
async fn validate_rejects_unknown_session() {
    let svc = test_service();
    let err = svc.validate_session("nope").await.unwrap_err();
    assert!(matches!(err, SessionServiceError::SessionNotFound));
}

#[tokio::test]
async fn logout_invalidates_session() {
    let svc = test_service();
    let session = svc
        .issue_session_for(
            UserId::new(1),
            Some(PlatformId::TWITCH),
            Some("123"),
            None,
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    svc.logout(session.token.as_str()).await.unwrap();
    let err = svc
        .validate_session(session.token.as_str())
        .await
        .unwrap_err();
    assert!(matches!(err, SessionServiceError::SessionNotFound));
}

#[tokio::test]
async fn expired_sessions_are_pruned() {
    let repo = Arc::new(InMemorySessionRepository::new());
    let svc = SessionService::new(
        Arc::clone(&repo),
        Arc::new(AdminService::new(Arc::new(InMemoryAdminRepository::new()))),
        test_settings(60),
    );

    let fresh = svc
        .issue_session_for(
            UserId::new(1),
            Some(PlatformId::TWITCH),
            Some("123"),
            None,
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    let stale = Session::new(
        SessionToken::new("stale"),
        UserId::new(9),
        Some(PlatformId::TWITCH),
        Some("9".to_string()),
        None,
        Utc::now() - Duration::from_secs(3600),
        Utc::now() - Duration::from_secs(60),
    );
    repo.save_session(&stale).await.unwrap();

    assert_eq!(svc.prune_expired().await.unwrap(), 1);
    assert!(svc.validate_session(fresh.token.as_str()).await.is_ok());
    let err = svc.validate_session("stale").await.unwrap_err();
    assert!(matches!(err, SessionServiceError::SessionNotFound));
}

#[tokio::test]
async fn session_ttl_follows_live_settings() {
    let settings = SharedSettings::test_new(RuntimeConfig::test_runtime("test-key"));
    let svc = SessionService::new(
        Arc::new(InMemorySessionRepository::new()),
        Arc::new(AdminService::new(Arc::new(InMemoryAdminRepository::new()))),
        settings.clone(),
    );

    assert_eq!(svc.ttl(), Duration::from_secs(24 * 60 * 60));

    let mut next = RuntimeConfig::test_runtime("test-key");
    next.session.ttl_secs = 30;
    settings.set(next);

    assert_eq!(svc.ttl(), Duration::from_secs(30));
}
