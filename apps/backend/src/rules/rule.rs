use crate::actions::ActionId;
use crate::platform::PlatformId;
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

    pub fn referenced_reward_ids(&self) -> Vec<&str> {
        match &self.conditions {
            RuleConditions::RewardRedemption(RewardConditions { reward_ids, .. }) => {
                reward_ids.iter().map(String::as_str).collect()
            }
            _ => Vec::new(),
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
    pub fn new(reward_ids: Vec<String>, platform: Option<PlatformId>) -> Self {
        Self {
            reward_ids,
            platform,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RewardConditions {
    pub reward_ids: Vec<String>,
    pub platform: Option<PlatformId>,
    #[serde(skip)]
    _sealed: (),
}

impl<'de> Deserialize<'de> for RewardConditions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            reward_id: Option<String>,
            #[serde(default)]
            reward_ids: Vec<String>,
            platform: Option<PlatformId>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let mut reward_ids = raw.reward_ids;
        if let Some(legacy) = raw.reward_id
            && !reward_ids.contains(&legacy)
        {
            reward_ids.push(legacy);
        }
        Ok(RewardConditions {
            reward_ids,
            platform: raw.platform,
            _sealed: (),
        })
    }
}

#[cfg(test)]
#[path = "rule.test.rs"]
mod tests;
