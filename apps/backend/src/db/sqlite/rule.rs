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
    <&'static str>::from(&trigger)
}

fn trigger_from_str(trigger: &str) -> Result<RuleTrigger, RepositoryError> {
    RuleTrigger::try_from(trigger)
        .map_err(|_| RepositoryError::Database(format!("invalid rule trigger: {trigger}")))
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
        let trigger = trigger_from_str(&self.trigger_kind)?;
        Ok(Rule::new(
            RuleId::new(self.id as u32),
            self.name,
            self.enabled,
            trigger,
            parse_conditions(&self.conditions)?,
            ActionId::new(self.action_id as u32),
            self.created_at,
            self.updated_at,
        ))
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
        Ok(Rule::new(
            RuleId::new(row.id as u32),
            name.to_string(),
            enabled,
            trigger,
            conditions,
            action_id,
            now,
            now,
        ))
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
        Ok(row.map(|row| {
            Rule::new(
                rule.id,
                rule.name.clone(),
                rule.enabled,
                rule.trigger,
                rule.conditions.clone(),
                rule.action_id,
                row.created_at,
                now,
            )
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
#[path = "rule.test.rs"]
mod tests;
