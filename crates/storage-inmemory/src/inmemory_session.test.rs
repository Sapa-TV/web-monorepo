use std::time::Duration;

use super::*;

#[tokio::test]
async fn session_roundtrip() {
    let repo = InMemorySessionRepository::new();
    let session = Session::new(
        SessionToken::new("tok"),
        UserId::new(1),
        Some("123".to_string()),
        Some("tester".to_string()),
        Utc::now(),
        Utc::now() + Duration::from_secs(3600),
    );
    repo.save_session(&session).await.unwrap();

    let fetched = repo.get_session(&session.token).await.unwrap().unwrap();
    assert_eq!(fetched.user_id, UserId::new(1));
    assert_eq!(fetched.twitch_user_id.as_deref(), Some("123"));

    assert!(repo.delete_session(&session.token).await.unwrap());
    assert!(!repo.delete_session(&session.token).await.unwrap());
}

#[tokio::test]
async fn reassign_moves_every_session_of_user() {
    let repo = InMemorySessionRepository::new();
    let now = Utc::now();
    for token in ["a", "b"] {
        let session = Session::new(
            SessionToken::new(token),
            UserId::new(1),
            None,
            None,
            now,
            now + Duration::from_secs(3600),
        );
        repo.save_session(&session).await.unwrap();
    }

    assert_eq!(
        repo.reassign_user(UserId::new(1), UserId::new(2))
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        repo.get_session(&SessionToken::new("a"))
            .await
            .unwrap()
            .unwrap()
            .user_id,
        UserId::new(2)
    );
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
        UserId::new(1),
        Some("1".to_string()),
        None,
        now,
        now - Duration::from_secs(1),
    );
    let fresh = Session::new(
        SessionToken::new("fresh"),
        UserId::new(2),
        Some("2".to_string()),
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
