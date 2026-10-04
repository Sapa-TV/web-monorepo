use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};

use crate::actions::action::ActionKind;
use crate::actions::service::ActionService;
use crate::db::inmemory_actions::InMemoryActionRepository;
use crate::db::inmemory_rules::InMemoryRuleRepository;
use crate::ingress::event::{PlatformEvent, RuleTrigger};
use crate::platform::PlatformId;
use crate::rules::rule::{RuleConditions, RuleId};

use super::*;

type TestActions = ActionService<InMemoryActionRepository>;
type TestRules = RuleService<InMemoryRuleRepository, InMemoryActionRepository>;
type TestEngine = RuleEngine<InMemoryRuleRepository, InMemoryActionRepository>;

async fn setup() -> (Arc<TestRules>, Arc<TestActions>) {
    let actions = Arc::new(TestActions::new(Arc::new(InMemoryActionRepository::new())));
    let rules = Arc::new(RuleService::new(
        Arc::new(InMemoryRuleRepository::new()),
        Arc::clone(&actions),
    ));
    (rules, actions)
}

fn chat_event(user_id: &str, text: &str) -> Arc<PlatformEvent> {
    Arc::new(PlatformEvent::chat_message(
        PlatformId::TWITCH,
        "msg-1",
        user_id.to_string(),
        "viewer".to_string(),
        text.to_string(),
    ))
}

fn reward_event() -> Arc<PlatformEvent> {
    Arc::new(PlatformEvent::reward_redemption(
        PlatformId::TWITCH,
        "red-1",
        "1".to_string(),
        "viewer".to_string(),
        "reward-9".to_string(),
        "Spin".to_string(),
        500,
        "please".to_string(),
        "unfulfilled".to_string(),
    ))
}

async fn seed_action(actions: &TestActions, kind: ActionKind) -> ActionId {
    actions.create("test", kind, true).await.unwrap().id
}

fn chat_conditions() -> RuleConditions {
    RuleConditions::ChatMessage(MessageConditions::new(
        MessageMatcher::Contains,
        Some("!spin".into()),
    ))
}

#[test]
fn chat_matcher_contains() {
    let conditions = chat_conditions();
    assert!(conditions_match(
        &conditions,
        &chat_event("1", "hey !spin please").payload
    ));
    assert!(!conditions_match(
        &conditions,
        &chat_event("1", "hey").payload
    ));
}

#[test]
fn chat_matcher_equals() {
    let conditions = RuleConditions::ChatMessage(MessageConditions::new(
        MessageMatcher::Equals,
        Some("!spin".into()),
    ));
    assert!(conditions_match(
        &conditions,
        &chat_event("1", "!spin").payload
    ));
    assert!(!conditions_match(
        &conditions,
        &chat_event("1", "!spin ").payload
    ));
}

#[test]
fn chat_matcher_missing_pattern_matches_nothing() {
    let conditions =
        RuleConditions::ChatMessage(MessageConditions::new(MessageMatcher::Contains, None));
    assert!(!conditions_match(
        &conditions,
        &chat_event("1", "!spin").payload
    ));
}

#[test]
fn reward_conditions_match_by_id() {
    let expected = RuleConditions::RewardRedemption(RewardConditions::new(Some("reward-9".into())));
    let wildcard = RuleConditions::RewardRedemption(RewardConditions::new(None));
    assert!(conditions_match(&expected, &reward_event().payload));
    assert!(conditions_match(&wildcard, &reward_event().payload));
}

#[test]
fn mismatched_trigger_never_matches() {
    assert!(!conditions_match(
        &chat_conditions(),
        &reward_event().payload
    ));
}

#[tokio::test]
async fn engine_forwards_matching_event_only() {
    let (rules, actions) = setup().await;
    let action_id = seed_action(&actions, ActionKind::EnqueueRoulette).await;
    rules
        .create(
            "spin",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(),
            action_id,
        )
        .await
        .unwrap();

    let engine = TestEngine::new(Arc::clone(&rules), Arc::clone(&actions));
    let (tx, mut rx) = mpsc::channel(1);
    let (btx, brx) = broadcast::channel(1);
    let handle = tokio::spawn(async move { engine.run(brx, tx).await });

    btx.send(chat_event("1", "!spin now")).unwrap();
    assert!(rx.recv().await.is_some());

    btx.send(chat_event("1", "plain")).unwrap();
    assert!(rx.try_recv().is_err(), "non-matching event must be skipped");

    drop(btx);
    handle.await.unwrap();
}

#[tokio::test]
async fn engine_reloads_after_rule_disabled() {
    let (rules, actions) = setup().await;
    let action_id = seed_action(&actions, ActionKind::EnqueueRoulette).await;
    let rule = rules
        .create(
            "spin",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(),
            action_id,
        )
        .await
        .unwrap();

    let engine = TestEngine::new(Arc::clone(&rules), Arc::clone(&actions));
    let (tx, mut rx) = mpsc::channel(1);
    let (btx, brx) = broadcast::channel(1);
    let handle = tokio::spawn(async move { engine.run(brx, tx).await });

    btx.send(chat_event("1", "!spin")).unwrap();
    assert!(rx.recv().await.is_some());

    rules.update(rule.with_enabled(false)).await.unwrap();

    btx.send(chat_event("1", "!spin")).unwrap();
    assert!(rx.try_recv().is_err(), "disabled rule must not fire");

    drop(btx);
    handle.await.unwrap();
}

#[tokio::test]
async fn engine_reloads_after_action_deleted() {
    let (rules, actions) = setup().await;
    let action = actions
        .create("reply", ActionKind::NoAction, true)
        .await
        .unwrap();
    rules
        .create(
            "spin",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(),
            action.id,
        )
        .await
        .unwrap();

    let engine = TestEngine::new(Arc::clone(&rules), Arc::clone(&actions));
    let (tx, mut rx) = mpsc::channel(1);
    let (btx, brx) = broadcast::channel(1);
    let handle = tokio::spawn(async move { engine.run(brx, tx).await });

    btx.send(chat_event("1", "!spin")).unwrap();
    assert!(rx.recv().await.is_some());

    actions.delete(action.id).await.unwrap();

    btx.send(chat_event("1", "!spin")).unwrap();
    assert!(rx.try_recv().is_err(), "deleted action must not fire");

    drop(btx);
    handle.await.unwrap();
}

#[test]
fn rule_id_repr_stable() {
    assert_eq!(RuleId::new(3).to_string(), "3");
}
