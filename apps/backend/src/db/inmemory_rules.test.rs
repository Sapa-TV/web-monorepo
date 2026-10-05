use super::*;
use crate::rules::rule::{MessageConditions, MessageMatcher, RewardConditions};

fn conditions(trigger: RuleTrigger) -> RuleConditions {
    match trigger {
        RuleTrigger::ChatMessage => RuleConditions::ChatMessage(MessageConditions::new(
            MessageMatcher::Contains,
            Some("!spin".to_string()),
        )),
        RuleTrigger::RewardRedemption => {
            RuleConditions::RewardRedemption(RewardConditions::new(Vec::new(), None))
        }
    }
}

#[tokio::test]
async fn create_and_get() {
    let repo = InMemoryRuleRepository::new();
    let rule = repo
        .create(
            "chat-spin",
            true,
            RuleTrigger::ChatMessage,
            conditions(RuleTrigger::ChatMessage),
            ActionId::new(1),
        )
        .await
        .unwrap();
    assert_eq!(rule.id, RuleId::new(1));

    let fetched = repo.get_by_id(RuleId::new(1)).await.unwrap().unwrap();
    assert_eq!(fetched.name, "chat-spin");
    assert_eq!(fetched.action_id, ActionId::new(1));
}

#[tokio::test]
async fn list_returns_all() {
    let repo = InMemoryRuleRepository::new();
    repo.create(
        "a",
        true,
        RuleTrigger::ChatMessage,
        conditions(RuleTrigger::ChatMessage),
        ActionId::new(1),
    )
    .await
    .unwrap();
    repo.create(
        "b",
        false,
        RuleTrigger::RewardRedemption,
        conditions(RuleTrigger::RewardRedemption),
        ActionId::new(2),
    )
    .await
    .unwrap();
    assert_eq!(repo.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn update_replaces_fields_and_touches_updated_at() {
    let repo = InMemoryRuleRepository::new();
    repo.create(
        "a",
        true,
        RuleTrigger::ChatMessage,
        conditions(RuleTrigger::ChatMessage),
        ActionId::new(1),
    )
    .await
    .unwrap();
    let original = repo.get_by_id(RuleId::new(1)).await.unwrap().unwrap();
    let updated = repo
        .update(original.clone().with_name("renamed").with_enabled(false))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "renamed");
    assert!(!updated.enabled);
    assert_eq!(updated.created_at, original.created_at);
    assert!(updated.updated_at > original.updated_at);

    assert!(
        repo.update(original.with_id(RuleId::new(99)))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn delete_removes_entry() {
    let repo = InMemoryRuleRepository::new();
    repo.create(
        "a",
        true,
        RuleTrigger::ChatMessage,
        conditions(RuleTrigger::ChatMessage),
        ActionId::new(1),
    )
    .await
    .unwrap();
    assert!(repo.delete(RuleId::new(1)).await.unwrap());
    assert!(!repo.delete(RuleId::new(1)).await.unwrap());
    assert!(repo.get_by_id(RuleId::new(1)).await.unwrap().is_none());
}
