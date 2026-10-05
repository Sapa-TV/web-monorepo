use std::time::Duration;

use tokio::time::sleep;

use crate::actions::action::{Action, ActionKind};
use crate::db::sqlite::test_pool;
use crate::rules::rule::{MessageConditions, MessageMatcher, RewardConditions};

use super::*;

async fn repos() -> (SqliteRuleRepository, SqliteActionRepo) {
    let (pool, _path) = test_pool().await;
    (
        SqliteRuleRepository::new(pool.clone()),
        SqliteActionRepo::new(pool),
    )
}

struct SqliteActionRepo(SqlitePool);

impl SqliteActionRepo {
    fn new(pool: SqlitePool) -> Self {
        Self(pool)
    }

    async fn create(&self, name: &str) -> Action {
        use crate::actions::repository::ActionRepository;
        use crate::db::sqlite::action::SqliteActionRepository;
        SqliteActionRepository::new(self.0.clone())
            .create(name, ActionKind::NoAction, true)
            .await
            .unwrap()
    }
    async fn delete(&self, id: ActionId) -> bool {
        use crate::actions::repository::ActionRepository;
        use crate::db::sqlite::action::SqliteActionRepository;
        SqliteActionRepository::new(self.0.clone())
            .delete(id)
            .await
            .unwrap()
    }
}

fn chat_conditions() -> RuleConditions {
    RuleConditions::ChatMessage(MessageConditions::new(
        MessageMatcher::Contains,
        Some("!spin".to_string()),
    ))
}

#[tokio::test]
async fn create_and_get_roundtrip_both_triggers() {
    let (repo, actions) = repos().await;
    let action = actions.create("spin").await;

    for (name, trigger, conditions) in [
        ("chat-spin", RuleTrigger::ChatMessage, chat_conditions()),
        (
            "reward",
            RuleTrigger::RewardRedemption,
            RuleConditions::RewardRedemption(RewardConditions::new(
                vec!["rew-1".to_string()],
                None,
            )),
        ),
    ] {
        let saved = repo
            .create(name, true, trigger, conditions.clone(), action.id)
            .await
            .unwrap();
        assert_eq!(saved.created_at, saved.updated_at);

        let fetched = repo.get_by_id(saved.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, name);
        assert_eq!(fetched.trigger, trigger);
        assert_eq!(fetched.conditions, conditions);
        assert_eq!(fetched.action_id, action.id);
    }

    assert!(repo.get_by_id(RuleId::new(999)).await.unwrap().is_none());
}

#[tokio::test]
async fn list_returns_in_insertion_order() {
    let (repo, actions) = repos().await;
    assert!(repo.list().await.unwrap().is_empty());

    let action = actions.create("a").await;
    repo.create(
        "r1",
        true,
        RuleTrigger::ChatMessage,
        chat_conditions(),
        action.id,
    )
    .await
    .unwrap();
    repo.create(
        "r2",
        false,
        RuleTrigger::RewardRedemption,
        RuleConditions::RewardRedemption(RewardConditions::new(Vec::new(), None)),
        action.id,
    )
    .await
    .unwrap();

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].name, "r1");
    assert_eq!(all[1].name, "r2");
    assert!(!all[1].enabled);
}

#[tokio::test]
async fn update_preserves_created_at_and_missing_is_none() {
    let (repo, actions) = repos().await;
    let action = actions.create("a").await;
    let saved = repo
        .create(
            "old",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(),
            action.id,
        )
        .await
        .unwrap();
    sleep(Duration::from_millis(5)).await;

    let mut next = saved.clone();
    next.name = "renamed".to_string();
    next.conditions = RuleConditions::RewardRedemption(RewardConditions::new(Vec::new(), None));
    let updated = repo.update(next).await.unwrap().unwrap();

    assert_eq!(updated.name, "renamed");
    assert_eq!(
        updated.conditions,
        RuleConditions::RewardRedemption(RewardConditions::new(Vec::new(), None))
    );
    assert_eq!(updated.created_at, saved.created_at);
    assert!(updated.updated_at > saved.updated_at);

    let mut missing = saved.clone();
    missing.id = RuleId::new(999);
    assert!(repo.update(missing).await.unwrap().is_none());
}

#[tokio::test]
async fn deleting_action_cascades_to_its_rules() {
    let (repo, actions) = repos().await;
    let action = actions.create("doomed").await;
    let rule = repo
        .create(
            "bound",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(),
            action.id,
        )
        .await
        .unwrap();

    assert!(actions.delete(action.id).await);

    assert!(repo.get_by_id(rule.id).await.unwrap().is_none());
    assert!(repo.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn delete_reports_presence() {
    let (repo, actions) = repos().await;
    let action = actions.create("a").await;
    let rule = repo
        .create(
            "x",
            true,
            RuleTrigger::ChatMessage,
            chat_conditions(),
            action.id,
        )
        .await
        .unwrap();

    assert!(repo.delete(rule.id).await.unwrap());
    assert!(!repo.delete(rule.id).await.unwrap());
}
