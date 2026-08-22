use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::actions::ActionId;
use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::ingress::event::RuleTrigger;
use crate::rules::repository::RuleRepository;
use crate::rules::rule::{Rule, RuleConditions, RuleId};

#[non_exhaustive]
pub struct SqliteRuleRepository {
    pool: SqlitePool,
}

impl SqliteRuleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn trigger_to_str(trigger: RuleTrigger) -> &'static str {
    match trigger {
        RuleTrigger::ChatMessage => "chat_message",
        RuleTrigger::RewardRedemption => "reward_redemption",
    }
}

fn parse_conditions(conditions: &str) -> Result<RuleConditions, RepositoryError> {
    serde_json::from_str(conditions)
        .map_err(|e| RepositoryError::Database(format!("invalid rule conditions: {e}")))
}

struct RuleRow {
    id: i64,
    name: String,
    enabled: bool,
    trigger_kind: String,
    conditions: String,
    action_id: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl RuleRow {
    fn into_rule(self) -> Result<Rule, RepositoryError> {
        let trigger = match self.trigger_kind.as_str() {
            "chat_message" => RuleTrigger::ChatMessage,
            "reward_redemption" => RuleTrigger::RewardRedemption,
            other => {
                return Err(RepositoryError::Database(format!(
                    "invalid rule trigger: {other}"
                )));
            }
        };
        Ok(Rule {
            id: RuleId::new(self.id as u32),
            name: self.name,
            enabled: self.enabled,
            trigger,
            conditions: parse_conditions(&self.conditions)?,
            action_id: ActionId::new(self.action_id as u32),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

impl RuleRepository for SqliteRuleRepository {
    async fn create(
        &self,
        name: &str,
        enabled: bool,
        trigger: RuleTrigger,
        conditions: RuleConditions,
        action_id: ActionId,
    ) -> Result<Rule, RepositoryError> {
        let now = Utc::now();
        let conditions_json = serde_json::to_string(&conditions).map_err(|e| {
            RepositoryError::Database(format!("serialize rule conditions failed: {e}"))
        })?;
        let row = sqlx::query!(
            "INSERT INTO rules (name, enabled, trigger_kind, conditions, action_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING id AS \"id!: i64\"",
            name,
            enabled,
            trigger_to_str(trigger),
            conditions_json,
            action_id.get(),
            now,
            now
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(Rule {
            id: RuleId::new(row.id as u32),
            name: name.to_string(),
            enabled,
            trigger,
            conditions,
            action_id,
            created_at: now,
            updated_at: now,
        })
    }

    async fn get_by_id(&self, id: RuleId) -> Result<Option<Rule>, RepositoryError> {
        let row = sqlx::query_as!(
            RuleRow,
            r#"SELECT id AS "id!: i64", name, enabled AS "enabled: bool", trigger_kind, conditions, action_id AS "action_id!: i64", created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>"
               FROM rules WHERE id = ?"#,
            id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        match row {
            Some(row) => Ok(Some(row.into_rule()?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<Rule>, RepositoryError> {
        let rows = sqlx::query_as!(
            RuleRow,
            r#"SELECT id AS "id!: i64", name, enabled AS "enabled: bool", trigger_kind, conditions, action_id AS "action_id!: i64", created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>"
               FROM rules ORDER BY id"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        rows.into_iter()
            .map(|row| row.into_rule())
            .collect::<Result<Vec<_>, _>>()
    }

    async fn update(&self, rule: Rule) -> Result<Option<Rule>, RepositoryError> {
        let now = Utc::now();
        let conditions_json = serde_json::to_string(&rule.conditions).map_err(|e| {
            RepositoryError::Database(format!("serialize rule conditions failed: {e}"))
        })?;
        let row = sqlx::query!(
            r#"UPDATE rules SET name = ?1, enabled = ?2, trigger_kind = ?3, conditions = ?4, action_id = ?5, updated_at = ?6
               WHERE id = ?7
               RETURNING created_at AS "created_at: DateTime<Utc>""#,
            rule.name,
            rule.enabled,
            trigger_to_str(rule.trigger),
            conditions_json,
            rule.action_id.get(),
            now,
            rule.id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| Rule {
            id: rule.id,
            name: rule.name.clone(),
            enabled: rule.enabled,
            trigger: rule.trigger,
            conditions: rule.conditions.clone(),
            action_id: rule.action_id,
            created_at: row.created_at,
            updated_at: now,
        }))
    }

    async fn delete(&self, id: RuleId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM rules WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
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
        RuleConditions::ChatMessage(MessageConditions {
            matcher: MessageMatcher::Contains,
            pattern: Some("!spin".to_string()),
        })
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
                RuleConditions::RewardRedemption(RewardConditions {
                    reward_id: Some("rew-1".to_string()),
                }),
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
            RuleConditions::RewardRedemption(RewardConditions { reward_id: None }),
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
        next.conditions = RuleConditions::RewardRedemption(RewardConditions { reward_id: None });
        let updated = repo.update(next).await.unwrap().unwrap();

        assert_eq!(updated.name, "renamed");
        assert_eq!(
            updated.conditions,
            RuleConditions::RewardRedemption(RewardConditions { reward_id: None })
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
}
