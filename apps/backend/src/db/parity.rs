//! Shared behavioral suites run against both repository implementations.
//! A suite may only use trait methods: it is the executable specification
//! of the contract both impls must satisfy.
//!
//! Deliberately out of scope (see docs/plan-sqlite-repositories.md):
//! FK cascades (sqlite-only), seeded data (different sources per impl),
//! storage internals.

#![cfg(test)]

use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use tokio::time::sleep;

use crate::actions::action::{ActionId, ActionKind};
use crate::actions::repository::ActionRepository;
use crate::admin::repository::AdminRepository;
use crate::config::repository::ConfigRepository;
use crate::config::runtime::RuntimeConfig;
use crate::db::inmemory_actions::InMemoryActionRepository;
use crate::db::inmemory_admin::InMemoryAdminRepository;
use crate::db::inmemory_config::InMemoryConfigRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_rules::InMemoryRuleRepository;
use crate::db::inmemory_session::InMemorySessionRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::db::sqlite::action::SqliteActionRepository;
use crate::db::sqlite::admin::SqliteAdminRepository;
use crate::db::sqlite::config::SqliteConfigRepository;
use crate::db::sqlite::queue::SqliteQueueRepository;
use crate::db::sqlite::rarity::SqliteRarityRepository;
use crate::db::sqlite::roulette_slot::SqliteRouletteSlotRepository;
use crate::db::sqlite::rule::SqliteRuleRepository;
use crate::db::sqlite::session::SqliteSessionRepository;
use crate::db::sqlite::test_pool;
use crate::db::sqlite::user::SqliteUserRepository;
use crate::error::RepositoryError;
use crate::ingress::event::RuleTrigger;
use crate::platform::PlatformId;
use crate::queue::entry::{QueueEntryId, QueueStatus};
use crate::queue::repository::{DequeueOutcome, QueueRepository, StatusUpdateOutcome};
use crate::roulette::rarity::{Rarity, RarityId, RarityRepository};
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::rules::repository::RuleRepository;
use crate::rules::rule::{MessageConditions, MessageMatcher, RewardConditions, RuleConditions};
use crate::rules::rule::{Rule, RuleId};
use crate::session::repository::SessionRepository;
use crate::session::{LoginTicket, LoginTicketToken, Session, SessionToken};
use crate::user::UserId;
use crate::user::repository::UserRepository;

// ---------------------------------------------------------------- config

async fn suite_config_lifecycle<C: ConfigRepository>(repo: &C) {
    assert!(repo.load().await.unwrap().is_none());

    let a = RuntimeConfig::test_runtime("a");
    repo.save(&a).await.unwrap();
    assert_eq!(repo.load().await.unwrap(), Some(a));

    let b = RuntimeConfig::test_runtime("b");
    repo.save(&b).await.unwrap();
    assert_eq!(repo.load().await.unwrap().unwrap().widget_access_key, "b");
}

#[tokio::test]
async fn parity_config_in_memory() {
    suite_config_lifecycle(&InMemoryConfigRepository::new()).await;
}

#[tokio::test]
async fn parity_config_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_config_lifecycle(&SqliteConfigRepository::new(pool)).await;
}

// ---------------------------------------------------------------- admin

async fn suite_admin_lifecycle<A: AdminRepository>(repo: &A) {
    assert!(repo.list().await.unwrap().is_empty());

    let created = repo.create("100", Some("sap"), true).await.unwrap();
    assert_eq!(created.twitch_id, "100");

    let err = repo.create("100", None, false).await.unwrap_err();
    assert!(matches!(err, RepositoryError::Conflict(_)));

    let renamed = repo
        .update_display_name("100", "renamed")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(renamed.display_name.as_deref(), Some("renamed"));
    assert!(
        repo.update_display_name("404", "x")
            .await
            .unwrap()
            .is_none()
    );

    let demoted = repo.set_root("100", false).await.unwrap().unwrap();
    assert!(!demoted.is_root);
    assert!(repo.set_root("404", true).await.unwrap().is_none());

    assert_eq!(repo.list().await.unwrap().len(), 1);

    assert!(repo.delete_by_twitch_id("100").await.unwrap());
    assert!(!repo.delete_by_twitch_id("100").await.unwrap());
    assert!(repo.get_by_twitch_id("100").await.unwrap().is_none());
}

#[tokio::test]
async fn parity_admin_in_memory() {
    suite_admin_lifecycle(&InMemoryAdminRepository::new()).await;
}

#[tokio::test]
async fn parity_admin_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_admin_lifecycle(&SqliteAdminRepository::new(pool)).await;
}

// ---------------------------------------------------------------- session

async fn suite_session_lifecycle<S: SessionRepository>(repo: &S) {
    let now = Utc::now();

    for (token, ttl_secs) in [("expired", -3600i64), ("edge", 0), ("fresh", 3600)] {
        let session = Session::new(
            SessionToken::new(token),
            token.to_string(),
            None,
            now,
            now + ChronoDuration::seconds(ttl_secs),
        );
        repo.save_session(&session).await.unwrap();
    }

    // upsert same token replaces data
    let fresh = Session::new(
        SessionToken::new("fresh"),
        "42".to_string(),
        Some("n".to_string()),
        now,
        now + ChronoDuration::hours(2),
    );
    repo.save_session(&fresh).await.unwrap();
    let fetched = repo.get_session(&fresh.token).await.unwrap().unwrap();
    assert_eq!(fetched.twitch_user_id, "42");

    // destructive ticket take
    let ticket = LoginTicket::new(
        LoginTicketToken::new("tic"),
        "7".to_string(),
        None,
        now,
        now + ChronoDuration::seconds(600),
    );
    repo.save_ticket(&ticket).await.unwrap();
    assert_eq!(
        repo.take_ticket(&ticket.ticket)
            .await
            .unwrap()
            .unwrap()
            .twitch_user_id,
        "7"
    );
    assert!(repo.take_ticket(&ticket.ticket).await.unwrap().is_none());

    // purge boundary is inclusive (expires_at <= now)
    assert_eq!(repo.purge_expired_sessions(now).await.unwrap(), 2);
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
    assert_eq!(repo.purge_expired_tickets(now).await.unwrap(), 0);

    assert!(repo.delete_session(&fresh.token).await.unwrap());
    assert!(!repo.delete_session(&fresh.token).await.unwrap());
    assert!(repo.get_session(&fresh.token).await.unwrap().is_none());
}

#[tokio::test]
async fn parity_session_in_memory() {
    suite_session_lifecycle(&InMemorySessionRepository::new()).await;
}

#[tokio::test]
async fn parity_session_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_session_lifecycle(&SqliteSessionRepository::new(pool)).await;
}

// ---------------------------------------------------------------- user

async fn suite_user_lifecycle<U: UserRepository>(repo: &U) {
    let first = repo.create("V1").await.unwrap();
    let second = repo.create("V2").await.unwrap();
    assert_eq!(first.id.get(), 1);
    assert_eq!(second.id.get(), 2);

    assert!(
        repo.find_by_platform(PlatformId::TWITCH, "t-1")
            .await
            .unwrap()
            .is_none()
    );

    let link = repo
        .link_platform(first.id, PlatformId::TWITCH, "t-1", "v_ttv")
        .await
        .unwrap();
    assert_eq!(link.platform_username, "v_ttv");

    let found = repo
        .find_by_platform(PlatformId::TWITCH, "t-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, first.id);

    let dup = repo
        .link_platform(second.id, PlatformId::TWITCH, "t-1", "x")
        .await
        .unwrap_err();
    assert!(matches!(dup, RepositoryError::Conflict(_)));
    assert_eq!(repo.get_platforms(second.id).await.unwrap().len(), 0);

    sleep(Duration::from_millis(5)).await;
    let renamed = repo
        .update_display_name(first.id, "New")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(renamed.display_name, "New");
    assert!(renamed.updated_at > first.updated_at);
    assert!(
        repo.update_display_name(UserId::new(999), "x")
            .await
            .unwrap()
            .is_none()
    );

    let relinked = repo
        .update_platform_username(first.id, PlatformId::TWITCH, "v2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(relinked.platform_username, "v2");
    assert!(
        repo.update_platform_username(second.id, PlatformId::TWITCH, "z")
            .await
            .unwrap()
            .is_none()
    );

    assert_eq!(repo.get_platforms(first.id).await.unwrap().len(), 1);

    assert!(
        !repo
            .delete_platform(second.id, PlatformId::TWITCH)
            .await
            .unwrap()
    );
    assert!(
        repo.delete_platform(first.id, PlatformId::TWITCH)
            .await
            .unwrap()
    );
    assert!(repo.get_platforms(first.id).await.unwrap().is_empty());

    assert!(repo.delete_user(first.id).await.unwrap());
    assert!(!repo.delete_user(first.id).await.unwrap());
}

#[tokio::test]
async fn parity_user_in_memory() {
    suite_user_lifecycle(&InMemoryUserRepository::new()).await;
}

#[tokio::test]
async fn parity_user_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_user_lifecycle(&SqliteUserRepository::new(pool)).await;
}

// ---------------------------------------------------------------- action

async fn suite_action_lifecycle<R: ActionRepository>(repo: &R) -> Vec<ActionId> {
    let mut ids = Vec::new();
    for (name, kind) in [
        ("noop", ActionKind::NoAction),
        ("spin", ActionKind::EnqueueRoulette),
        (
            "reply",
            ActionKind::ChatReply {
                message_template: "hi {username}".to_string(),
            },
        ),
    ] {
        let saved = repo.create(name, kind.clone(), true).await.unwrap();
        let fetched = repo.get_by_id(saved.id).await.unwrap().unwrap();
        assert_eq!(fetched.kind, kind);
        assert_eq!(fetched.created_at, fetched.updated_at);
        ids.push(saved.id);
    }

    assert_eq!(repo.list().await.unwrap().len(), ids.len());

    let first = repo.get_by_id(ids[0]).await.unwrap().unwrap();
    sleep(Duration::from_millis(5)).await;
    let mut next = first.clone();
    next.name = "renamed".to_string();
    next.enabled = false;
    let updated = repo.update(next).await.unwrap().unwrap();
    assert_eq!(updated.name, "renamed");
    assert!(!updated.enabled);
    assert_eq!(updated.created_at, first.created_at);
    assert!(updated.updated_at > first.updated_at);

    ids
}

#[tokio::test]
async fn parity_action_in_memory() {
    suite_action_lifecycle(&InMemoryActionRepository::new()).await;
}

#[tokio::test]
async fn parity_action_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_action_lifecycle(&SqliteActionRepository::new(pool)).await;
}

// ---------------------------------------------------------------- rule

async fn suite_rule_lifecycle<R: RuleRepository>(repo: &R, action_id: ActionId) {
    assert!(repo.list().await.unwrap().is_empty());

    let chat = RuleConditions::ChatMessage(MessageConditions::new(
        MessageMatcher::Contains,
        Some("!spin".to_string()),
    ));
    let reward = RuleConditions::RewardRedemption(RewardConditions::new(None));

    let r1 = repo
        .create(
            "chat-spin",
            true,
            RuleTrigger::ChatMessage,
            chat.clone(),
            action_id,
        )
        .await
        .unwrap();
    assert_eq!(r1.conditions, chat);

    let r2 = repo
        .create(
            "reward",
            false,
            RuleTrigger::RewardRedemption,
            reward.clone(),
            action_id,
        )
        .await
        .unwrap();

    let fetched = repo.get_by_id(r2.id).await.unwrap().unwrap();
    assert_eq!(fetched.conditions, reward);
    assert_eq!(fetched.trigger, RuleTrigger::RewardRedemption);

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].name, "chat-spin");
    assert_eq!(all[1].name, "reward");

    sleep(Duration::from_millis(5)).await;
    let mut next = r2.clone();
    next.name = "renamed".to_string();
    next.enabled = true;
    let updated = repo.update(next).await.unwrap().unwrap();
    assert_eq!(updated.name, "renamed");
    assert_eq!(updated.created_at, r2.created_at);
    assert!(updated.updated_at > r2.updated_at);

    let missing = Rule::new(
        RuleId::new(999),
        "missing".to_string(),
        true,
        RuleTrigger::ChatMessage,
        chat.clone(),
        action_id,
        Utc::now(),
        Utc::now(),
    );
    assert!(repo.update(missing).await.unwrap().is_none());

    assert!(repo.delete(r1.id).await.unwrap());
    assert!(!repo.delete(r1.id).await.unwrap());
    assert!(repo.get_by_id(r1.id).await.unwrap().is_none());
}

#[tokio::test]
async fn parity_rule_in_memory() {
    suite_rule_lifecycle(&InMemoryRuleRepository::new(), ActionId::new(42)).await;
}

#[tokio::test]
async fn parity_rule_sqlite() {
    let (pool, _path) = test_pool().await;
    let actions = SqliteActionRepository::new(pool.clone());
    let action = actions
        .create("for-rules", ActionKind::NoAction, true)
        .await
        .unwrap();
    suite_rule_lifecycle(&SqliteRuleRepository::new(pool), action.id).await;
}

// ---------------------------------------------------------------- rarity

/// Seed-agnostic: does not assume any particular rows exist upfront.
async fn suite_rarity_lifecycle<R: RarityRepository>(repo: &R) {
    let existing_ids: Vec<u32> = repo
        .load_all()
        .await
        .unwrap()
        .iter()
        .map(|r| r.id.get())
        .collect();

    let saved = repo
        .save(Rarity::new(
            RarityId::new(0),
            "suite",
            "Suite",
            "suite.png",
            "#123456",
        ))
        .await
        .unwrap();
    assert!(!existing_ids.contains(&saved.id.get()));

    let listed = repo.load_all().await.unwrap();
    assert!(listed.iter().any(|r| r.id == saved.id));

    let updated = repo
        .update(Rarity::new(
            saved.id,
            "suite",
            "Renamed",
            "suite.png",
            "#654321",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.display_name, "Renamed");

    let missing = RarityId::new(existing_ids.iter().max().copied().unwrap_or(0) + 10_000);
    assert!(
        repo.update(Rarity::new(missing, "m", "M", "m.png", "#000000"))
            .await
            .unwrap()
            .is_none()
    );

    assert!(repo.delete(saved.id).await.unwrap());
    assert!(
        !repo
            .load_all()
            .await
            .unwrap()
            .iter()
            .any(|r| r.id == saved.id)
    );
    assert!(!repo.delete(saved.id).await.unwrap());
}

#[tokio::test]
async fn parity_rarity_in_memory() {
    suite_rarity_lifecycle(&InMemoryRarityRepository::seed(vec![])).await;
}

#[tokio::test]
async fn parity_rarity_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_rarity_lifecycle(&SqliteRarityRepository::new(pool)).await;
}

// ---------------------------------------------------------------- slots

/// Seed-agnostic, like the rarity suite; out-of-range weights stay in the
/// sqlite-specific tests because the in-memory impl has no i64 constraint.
async fn suite_slot_lifecycle<R: RouletteSlotRepository>(repo: &R) {
    let existing_ids: Vec<u32> = repo
        .load_all()
        .await
        .unwrap()
        .iter()
        .map(|s| s.id.get())
        .collect();

    let saved = repo
        .save(RouletteSlot::new(
            RouletteSlotId::new(999_999),
            "suite-slot",
            RarityId::new(1),
            13,
            "chat",
        ))
        .await
        .unwrap();
    assert!(!existing_ids.contains(&saved.id.get()));
    assert_eq!(saved.weight, 13);

    let fetched = repo
        .load_all()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.id == saved.id)
        .unwrap();
    assert_eq!(fetched.name, "suite-slot");
    assert_eq!(fetched.rarity_id, RarityId::new(1));

    let updated = repo
        .update(RouletteSlot::new(
            saved.id,
            "suite-slot",
            RarityId::new(2),
            77,
            "jackpot",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.weight, 77);
    assert_eq!(updated.rarity_id, RarityId::new(2));

    let missing = RouletteSlotId::new(existing_ids.iter().max().copied().unwrap_or(0) + 10_000);
    assert!(
        repo.update(RouletteSlot::new(missing, "m", RarityId::new(1), 1, "chat"))
            .await
            .unwrap()
            .is_none()
    );

    assert!(repo.delete(saved.id).await.unwrap());
    assert!(!repo.delete(saved.id).await.unwrap());
}

#[tokio::test]
async fn parity_slot_in_memory() {
    suite_slot_lifecycle(&InMemoryRouletteSlotRepository::seed(vec![])).await;
}

#[tokio::test]
async fn parity_slot_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_slot_lifecycle(&SqliteRouletteSlotRepository::new(pool)).await;
}

// ---------------------------------------------------------------- queue

async fn suite_queue_priority_and_dequeue<R: QueueRepository>(repo: &R) {
    match repo
        .dequeue_next_with_slot(RouletteSlotId::new(1))
        .await
        .unwrap()
    {
        DequeueOutcome::Empty => {}
        _ => panic!("expected Empty"),
    }
    assert!(repo.peek_next().await.unwrap().is_none());

    let first = repo.enqueue(UserId::new(1), "a").await.unwrap();
    let second = repo.enqueue(UserId::new(2), "b").await.unwrap();
    repo.update_status_if(first.id, QueueStatus::Pending, QueueStatus::Error)
        .await
        .unwrap();

    let peeked = repo.peek_next().await.unwrap().unwrap();
    assert_eq!(peeked.id, first.id);

    match repo
        .dequeue_next_with_slot(RouletteSlotId::new(5))
        .await
        .unwrap()
    {
        DequeueOutcome::Picked(picked) => {
            assert_eq!(picked.id, first.id);
            assert_eq!(picked.status, QueueStatus::Spinning);
        }
        _ => panic!("expected Picked"),
    }

    match repo
        .dequeue_next_with_slot(RouletteSlotId::new(5))
        .await
        .unwrap()
    {
        DequeueOutcome::AlreadyActive => {}
        _ => panic!("expected AlreadyActive"),
    }

    repo.update_status_if(first.id, QueueStatus::Spinning, QueueStatus::Completed)
        .await
        .unwrap();
    match repo
        .dequeue_next_with_slot(RouletteSlotId::new(5))
        .await
        .unwrap()
    {
        DequeueOutcome::Picked(picked) => assert_eq!(picked.id, second.id),
        _ => panic!("expected second pending picked"),
    }
}

#[tokio::test]
async fn parity_queue_priority_in_memory() {
    suite_queue_priority_and_dequeue(&InMemoryQueueRepository::new()).await;
}

#[tokio::test]
async fn parity_queue_priority_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_queue_priority_and_dequeue(&SqliteQueueRepository::new(pool)).await;
}

async fn suite_queue_cas_outcomes<R: QueueRepository>(repo: &R) {
    let entry = repo.enqueue(UserId::new(1), "a").await.unwrap();

    match repo
        .update_status_if(entry.id, QueueStatus::Pending, QueueStatus::Spinning)
        .await
        .unwrap()
    {
        StatusUpdateOutcome::Updated(updated) => {
            assert_eq!(updated.status, QueueStatus::Spinning);
            assert!(updated.updated_at >= updated.created_at);
        }
        _ => panic!("expected Updated"),
    }

    match repo
        .update_status_if(entry.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap()
    {
        StatusUpdateOutcome::StatusMismatch => {}
        _ => panic!("expected StatusMismatch"),
    }

    match repo
        .update_status_if(
            QueueEntryId::new(999),
            QueueStatus::Pending,
            QueueStatus::Error,
        )
        .await
        .unwrap()
    {
        StatusUpdateOutcome::NotFound => {}
        _ => panic!("expected NotFound"),
    }
}

#[tokio::test]
async fn parity_queue_cas_in_memory() {
    suite_queue_cas_outcomes(&InMemoryQueueRepository::new()).await;
}

#[tokio::test]
async fn parity_queue_cas_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_queue_cas_outcomes(&SqliteQueueRepository::new(pool)).await;
}

async fn suite_queue_pagination<R: QueueRepository>(repo: &R) {
    assert!(repo.list(None, None, 100).await.unwrap().is_empty());

    for i in 0..5u32 {
        repo.enqueue(UserId::new(i + 1), &format!("u{i}"))
            .await
            .unwrap();
    }

    let page1 = repo.list(None, None, 2).await.unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].id.get(), 1);
    assert_eq!(page1[1].id.get(), 2);

    let page2 = repo.list(None, Some(page1[1].id), 2).await.unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].id.get(), 3);

    let page3 = repo.list(None, Some(page2[1].id), 2).await.unwrap();
    assert_eq!(page3.len(), 1);
    assert_eq!(page3[0].id.get(), 5);
}

#[tokio::test]
async fn parity_queue_pagination_in_memory() {
    suite_queue_pagination(&InMemoryQueueRepository::new()).await;
}

#[tokio::test]
async fn parity_queue_pagination_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_queue_pagination(&SqliteQueueRepository::new(pool)).await;
}

async fn suite_queue_purge_and_timeout<R: QueueRepository>(repo: &R) {
    let done = repo.enqueue(UserId::new(1), "done").await.unwrap();
    let cancelled = repo.enqueue(UserId::new(2), "cancelled").await.unwrap();
    let spun = repo.enqueue(UserId::new(3), "spun").await.unwrap();
    let pending = repo.enqueue(UserId::new(4), "pending").await.unwrap();

    repo.update_status_if(done.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap();
    repo.update_status_if(cancelled.id, QueueStatus::Pending, QueueStatus::Cancelled)
        .await
        .unwrap();
    repo.update_status_if(spun.id, QueueStatus::Pending, QueueStatus::Spinning)
        .await
        .unwrap();

    // mark_timed_out hits only Spinning, exactly once
    let future = Utc::now() + ChronoDuration::hours(1);
    let timed_out = repo.mark_timed_out(future).await.unwrap();
    assert_eq!(timed_out.len(), 1);
    assert_eq!(timed_out[0].id, spun.id);
    assert_eq!(timed_out[0].status, QueueStatus::Error);
    assert!(repo.mark_timed_out(future).await.unwrap().is_empty());

    // purge takes only aged Completed/Cancelled
    assert_eq!(repo.purge_completed_cancelled(future).await.unwrap(), 2);
    let remaining = repo.list(None, None, 100).await.unwrap();
    assert_eq!(remaining.len(), 2);

    // count_by_status reflects what survived
    let stats = repo.count_by_status().await.unwrap();
    assert_eq!(stats.pending, 1);
    assert_eq!(stats.error, 1);
    assert_eq!(stats.completed, 0);
    assert_eq!(stats.cancelled, 0);
    assert_eq!(stats.spinning, 0);

    let survived_ids: Vec<u32> = remaining.iter().map(|e| e.id.get()).collect();
    assert_eq!(survived_ids, vec![spun.id.get(), pending.id.get()]);
}

#[tokio::test]
async fn parity_queue_purge_and_timeout_in_memory() {
    suite_queue_purge_and_timeout(&InMemoryQueueRepository::new()).await;
}

#[tokio::test]
async fn parity_queue_purge_and_timeout_sqlite() {
    let (pool, _path) = test_pool().await;
    suite_queue_purge_and_timeout(&SqliteQueueRepository::new(pool)).await;
}
