use std::time::Duration;

use super::*;

#[tokio::test]
async fn session_roundtrip() {
    let repo = InMemorySessionRepository::new();
    let session = Session::new(
        SessionToken::new("tok"),
        "123".to_string(),
        Some("tester".to_string()),
        Utc::now(),
        Utc::now() + Duration::from_secs(3600),
    );
    repo.save_session(&session).await.unwrap();

    let fetched = repo.get_session(&session.token).await.unwrap().unwrap();
    assert_eq!(fetched.twitch_user_id, "123");

    assert!(repo.delete_session(&session.token).await.unwrap());
    assert!(!repo.delete_session(&session.token).await.unwrap());
}

#[tokio::test]
async fn ticket_take_is_destructive() {
    let repo = InMemorySessionRepository::new();
    let ticket = LoginTicket::new(
        LoginTicketToken::new("tic"),
        "123".to_string(),
        None,
        Utc::now(),
        Utc::now() + Duration::from_secs(600),
    );
    repo.save_ticket(&ticket).await.unwrap();

    let taken = repo.take_ticket(&ticket.ticket).await.unwrap().unwrap();
    assert_eq!(taken.twitch_user_id, "123");
    assert!(repo.take_ticket(&ticket.ticket).await.unwrap().is_none());
}

#[tokio::test]
async fn purge_removes_only_expired() {
    let repo = InMemorySessionRepository::new();
    let now = Utc::now();

    let stale = Session::new(
        SessionToken::new("stale"),
        "1".to_string(),
        None,
        now,
        now - Duration::from_secs(1),
    );
    let fresh = Session::new(
        SessionToken::new("fresh"),
        "2".to_string(),
        None,
        now,
        now + Duration::from_secs(1),
    );
    repo.save_session(&stale).await.unwrap();
    repo.save_session(&fresh).await.unwrap();

    assert_eq!(repo.purge_expired_sessions(now).await.unwrap(), 1);
    assert!(repo.get_session(&stale.token).await.unwrap().is_none());
    assert!(repo.get_session(&fresh.token).await.unwrap().is_some());
}
