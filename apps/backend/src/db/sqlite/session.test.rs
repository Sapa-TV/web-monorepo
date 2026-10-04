use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};

use crate::db::sqlite::test_pool;

use super::*;

async fn repo() -> SqliteSessionRepository {
    let (pool, _path) = test_pool().await;
    SqliteSessionRepository::new(pool)
}

fn sample_session(token: &str) -> Session {
    let now = Utc::now();
    Session::new(
        SessionToken::new(token),
        "123".to_string(),
        Some("tester".to_string()),
        now,
        now + Duration::from_secs(3600),
    )
}

#[tokio::test]
async fn session_roundtrip_and_double_delete() {
    let repo = repo().await;
    let session = sample_session("tok");

    repo.save_session(&session).await.unwrap();

    let fetched = repo.get_session(&session.token).await.unwrap().unwrap();
    assert_eq!(fetched.twitch_user_id, "123");
    assert_eq!(fetched.twitch_user_name.as_deref(), Some("tester"));

    assert!(repo.delete_session(&session.token).await.unwrap());
    assert!(!repo.delete_session(&session.token).await.unwrap());
    assert!(repo.get_session(&session.token).await.unwrap().is_none());
}

#[tokio::test]
async fn save_session_upserts_same_token() {
    let repo = repo().await;
    let mut session = sample_session("tok");

    repo.save_session(&session).await.unwrap();
    session.twitch_user_id = "456".to_string();
    repo.save_session(&session).await.unwrap();

    let fetched = repo.get_session(&session.token).await.unwrap().unwrap();
    assert_eq!(fetched.twitch_user_id, "456");
}

#[tokio::test]
async fn ticket_take_is_destructive() {
    let repo = repo().await;
    let now = Utc::now();
    let ticket = LoginTicket::new(
        LoginTicketToken::new("tic"),
        "123".to_string(),
        None,
        now,
        now + ChronoDuration::seconds(600),
    );

    repo.save_ticket(&ticket).await.unwrap();

    let taken = repo.take_ticket(&ticket.ticket).await.unwrap().unwrap();
    assert_eq!(taken.twitch_user_id, "123");
    assert!(taken.twitch_user_name.is_none());

    assert!(repo.take_ticket(&ticket.ticket).await.unwrap().is_none());
}

#[tokio::test]
async fn purge_removes_only_expired_boundary_inclusive() {
    let repo = repo().await;
    let now = Utc::now();

    for (token, ttl) in [
        ("expired", ChronoDuration::hours(-1)),
        ("boundary", ChronoDuration::zero()),
        ("fresh", ChronoDuration::hours(1)),
    ] {
        let session = Session::new(
            SessionToken::new(token),
            "1".to_string(),
            None,
            now,
            now + ttl,
        );
        repo.save_session(&session).await.unwrap();
    }

    let removed = repo.purge_expired_sessions(now).await.unwrap();

    assert_eq!(removed, 2);
    assert!(
        repo.get_session(&SessionToken::new("fresh"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        repo.get_session(&SessionToken::new("expired"))
            .await
            .unwrap()
            .is_none()
    );

    let removed_tickets = repo.purge_expired_tickets(now).await.unwrap();
    assert_eq!(removed_tickets, 0);
}
