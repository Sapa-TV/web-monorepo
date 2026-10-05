use crate::actions::action::ActionKind;
use crate::db::inmemory_actions::InMemoryActionRepository;
use crate::db::inmemory_rules::InMemoryRuleRepository;
use crate::rules::rule::{MessageConditions, RewardConditions};

use super::*;

fn test_actions() -> Arc<ActionService<InMemoryActionRepository>> {
    Arc::new(ActionService::new(
        Arc::new(InMemoryActionRepository::new()),
    ))
}

fn test_service(
    actions: &Arc<ActionService<InMemoryActionRepository>>,
) -> RuleService<InMemoryRuleRepository, InMemoryActionRepository> {
    RuleService::new(Arc::new(InMemoryRuleRepository::new()), Arc::clone(actions))
}

fn chat_conditions(matcher: MessageMatcher) -> RuleConditions {
    RuleConditions::ChatMessage(MessageConditions::new(matcher, Some("!spin".to_string())))
}

async fn seed_action(actions: &ActionService<InMemoryActionRepository>) -> ActionId {
    actions
        .create("enqueue", ActionKind::EnqueueRoulette, true)
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn create_validates_action_exists() {
    let actions = test_actions();
    let service = test_service(&actions);
    let err = service
        .create(
            "rule",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(MessageMatcher::Contains),
            ActionId::new(999),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, RuleServiceError::ActionNotFound));
}

#[tokio::test]
async fn create_requires_pattern_for_equals() {
    let actions = test_actions();
    let action_id = seed_action(&actions).await;
    let service = test_service(&actions);
    let err = service
        .create(
            "rule",
            true,
            RuleTrigger::ChatMessage,
            RuleConditions::ChatMessage(MessageConditions::new(MessageMatcher::Equals, None)),
            action_id,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, RuleServiceError::MissingPattern(_)));
}

#[tokio::test]
async fn create_bumps_and_seeds_cache() {
    let actions = test_actions();
    let action_id = seed_action(&actions).await;
    let service = test_service(&actions);
    let mut rx = service.subscribe_lifecycle();

    let rule = service
        .create(
            "rule",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(MessageMatcher::Contains),
            action_id,
        )
        .await
        .unwrap();
    rx.changed().await.unwrap();
    assert_eq!(rx.borrow_and_update().clone(), 1);

    let enabled = service.enabled_rules().await.unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].id, rule.id);
}

#[tokio::test]
async fn enabled_cache_refreshes_after_update() {
    let actions = test_actions();
    let action_id = seed_action(&actions).await;
    let service = test_service(&actions);
    let rule = service
        .create(
            "rule",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(MessageMatcher::Contains),
            action_id,
        )
        .await
        .unwrap();
    assert_eq!(service.enabled_rules().await.unwrap().len(), 1);

    service.update(rule.with_enabled(false)).await.unwrap();
    assert!(service.enabled_rules().await.unwrap().is_empty());
}

#[tokio::test]
async fn update_missing_is_not_found() {
    let actions = test_actions();
    let action_id = seed_action(&actions).await;
    let service = test_service(&actions);
    let err = service
        .update(rule_fixture(action_id).with_id(RuleId::new(999)))
        .await
        .unwrap_err();
    assert!(matches!(err, RuleServiceError::RuleNotFound));
}

fn rule_fixture(action_id: ActionId) -> Rule {
    Rule::new(
        RuleId::new(1),
        "x".to_string(),
        true,
        RuleTrigger::RewardRedemption,
        RuleConditions::RewardRedemption(RewardConditions::new(Vec::new(), None)),
        action_id,
        chrono::Utc::now(),
        chrono::Utc::now(),
    )
}

#[tokio::test]
async fn delete_missing_is_not_found() {
    let actions = test_actions();
    let service = test_service(&actions);
    let err = service.delete(RuleId::new(999)).await.unwrap_err();
    assert!(matches!(err, RuleServiceError::RuleNotFound));
}
