use crate::actions::ActionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

pub use crate::ingress::event::RuleTrigger;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct RuleId(u32);

impl RuleId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    pub enabled: bool,
    pub trigger: RuleTrigger,
    pub conditions: RuleConditions,
    pub action_id: ActionId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    _sealed: (),
}

impl Rule {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: RuleId,
        name: String,
        enabled: bool,
        trigger: RuleTrigger,
        conditions: RuleConditions,
        action_id: ActionId,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            enabled,
            trigger,
            conditions,
            action_id,
            created_at,
            updated_at,
            _sealed: (),
        }
    }

    pub fn referenced_reward_id(&self) -> Option<&str> {
        match &self.conditions {
            RuleConditions::RewardRedemption(RewardConditions {
                reward_id: Some(id),
                ..
            }) => Some(id),
            _ => None,
        }
    }

    pub fn with_id(mut self, id: RuleId) -> Self {
        self.id = id;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "trigger", rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuleConditions {
    ChatMessage(MessageConditions),
    RewardRedemption(RewardConditions),
}

impl MessageConditions {
    pub fn new(matcher: MessageMatcher, pattern: Option<String>) -> Self {
        Self {
            matcher,
            pattern,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct MessageConditions {
    pub matcher: MessageMatcher,
    pub pattern: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MessageMatcher {
    Contains,
    StartsWith,
    Equals,
    EndsWith,
}

impl RewardConditions {
    pub fn new(reward_id: Option<String>) -> Self {
        Self {
            reward_id,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct RewardConditions {
    pub reward_id: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_conditions() -> RuleConditions {
        RuleConditions::ChatMessage(MessageConditions::new(
            MessageMatcher::Contains,
            Some("!spin".to_string()),
        ))
    }

    #[test]
    fn conditions_serde_roundtrip() {
        let conditions = sample_conditions();
        let json = serde_json::to_value(&conditions).unwrap();
        let back: RuleConditions = serde_json::from_value(json).unwrap();
        assert_eq!(back, conditions);
    }

    #[test]
    fn conditions_tag_matches_trigger() {
        let chat = sample_conditions();
        let reward =
            RuleConditions::RewardRedemption(RewardConditions::new(Some("reward-1".to_string())));

        for (conditions, trigger) in [
            (chat, RuleTrigger::ChatMessage),
            (reward, RuleTrigger::RewardRedemption),
        ] {
            let json = serde_json::to_value(&conditions).unwrap();
            assert_eq!(json["trigger"], serde_json::to_value(trigger).unwrap());
        }
    }

    #[test]
    fn message_matcher_tagged_snake_case() {
        let contains =
            MessageConditions::new(MessageMatcher::StartsWith, Some("!spin".to_string()));
        let json = serde_json::to_value(contains).unwrap();
        assert_eq!(json["matcher"], "starts_with");
    }

    #[test]
    fn reward_conditions_serialize_with_trigger_tag() {
        let conditions = RuleConditions::RewardRedemption(RewardConditions::new(None));
        let json = serde_json::to_value(conditions).unwrap();
        assert_eq!(json["trigger"], "reward_redemption");
        assert!(json.get("reward_id").is_some());
    }
}
