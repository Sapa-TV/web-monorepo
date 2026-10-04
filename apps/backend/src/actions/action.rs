use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct ActionId(u32);

impl ActionId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Action {
    pub id: ActionId,
    pub name: String,
    pub kind: ActionKind,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    _sealed: (),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionKind {
    NoAction,
    EnqueueRoulette,
    ChatReply { message_template: String },
}

impl Action {
    pub fn new(
        id: ActionId,
        name: String,
        kind: ActionKind,
        enabled: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            enabled,
            created_at,
            updated_at,
            _sealed: (),
        }
    }

    pub fn noop(id: ActionId) -> Self {
        let now = Utc::now();
        Self::new(
            id,
            "no-op".to_string(),
            ActionKind::NoAction,
            true,
            now,
            now,
        )
    }
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct EventContext {
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    pub reward_title: String,
    pub reward_cost: i64,
    pub user_input: String,
    _sealed: (),
}

impl EventContext {
    #[must_use]
    pub fn chat(user_id: String, user_name: String, text: String) -> Self {
        Self {
            user_id,
            user_name,
            text,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn reward(
        user_id: String,
        user_name: String,
        reward_title: String,
        reward_cost: i64,
        user_input: String,
    ) -> Self {
        Self {
            user_id,
            user_name,
            reward_title,
            reward_cost,
            user_input,
            ..Self::default()
        }
    }
}

pub fn render(template: &str, ctx: &EventContext) -> String {
    template
        .replace("{username}", &ctx.user_name)
        .replace("{user_id}", &ctx.user_id)
        .replace("{text}", &ctx.text)
        .replace("{reward_title}", &ctx.reward_title)
        .replace("{cost}", &ctx.reward_cost.to_string())
        .replace("{user_input}", &ctx.user_input)
}

#[cfg(test)]
#[path = "action.test.rs"]
mod tests;
