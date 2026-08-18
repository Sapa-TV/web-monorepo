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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionKind {
    NoAction,
    EnqueueRoulette,
    ChatReply { message_template: String },
}

impl Action {
    pub fn noop(id: ActionId) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: "no-op".to_string(),
            kind: ActionKind::NoAction,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
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
mod tests {
    use chrono::Utc;

    use super::*;

    fn chat_context() -> EventContext {
        EventContext {
            user_id: "1".to_string(),
            user_name: "viewer".to_string(),
            text: "hello".to_string(),
            ..EventContext::default()
        }
    }

    fn reward_context() -> EventContext {
        EventContext {
            user_id: "1".to_string(),
            user_name: "viewer".to_string(),
            reward_title: "Spin".to_string(),
            reward_cost: 500,
            user_input: "please".to_string(),
            ..EventContext::default()
        }
    }

    #[test]
    fn render_replaces_known_keys() {
        assert_eq!(
            render("@{username} says {text}", &chat_context()),
            "@viewer says hello"
        );
        assert_eq!(
            render(
                "{username} {reward_title} for {cost} [{user_input}]",
                &reward_context()
            ),
            "viewer Spin for 500 [please]"
        );
        assert_eq!(render("{user_id}", &chat_context()), "1");
    }

    #[test]
    fn render_leaves_unknown_keys_untouched() {
        assert_eq!(
            render("hi {unknown} {foo}", &chat_context()),
            "hi {unknown} {foo}"
        );
        assert_eq!(
            render("{username {nested}}", &chat_context()),
            "{username {nested}}"
        );
    }

    #[test]
    fn render_empty_context() {
        assert_eq!(render("{username}", &EventContext::default()), "");
    }

    #[test]
    fn action_kind_serde_roundtrip() {
        for kind in [
            ActionKind::NoAction,
            ActionKind::EnqueueRoulette,
            ActionKind::ChatReply {
                message_template: "hi {username}".to_string(),
            },
        ] {
            let json = serde_json::to_value(&kind).unwrap();
            let back: ActionKind = serde_json::from_value(json).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn no_action_serializes_as_no_action() {
        let json = serde_json::to_value(ActionKind::NoAction).unwrap();
        assert_eq!(json["type"], "no_action");
        let back: ActionKind = serde_json::from_value(json).unwrap();
        assert_eq!(back, ActionKind::NoAction);
    }

    #[test]
    fn noop_action_builds_empty_kind() {
        let action = Action::noop(ActionId::new(1));
        assert_eq!(action.kind, ActionKind::NoAction);
        assert!(action.enabled);
    }

    #[test]
    fn action_kind_tagged_snake_case() {
        let kind = ActionKind::ChatReply {
            message_template: "hi {username}".to_string(),
        };
        let json = serde_json::to_value(kind).unwrap();
        assert_eq!(json["type"], "chat_reply");
        assert_eq!(json["message_template"], "hi {username}");

        let enqueue = ActionKind::EnqueueRoulette;
        let json = serde_json::to_value(enqueue).unwrap();
        assert_eq!(json["type"], "enqueue_roulette");
    }

    #[test]
    fn action_id_transparent_serde() {
        let id = ActionId::new(7);
        let json = serde_json::to_value(id).unwrap();
        assert_eq!(json, serde_json::json!(7));
        let back: ActionId = serde_json::from_value(json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn action_holds_kind() {
        let action = Action {
            id: ActionId::new(1),
            name: "reply".to_string(),
            kind: ActionKind::ChatReply {
                message_template: "hi {username}".to_string(),
            },
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(
            action.kind,
            ActionKind::ChatReply {
                message_template: "hi {username}".to_string()
            }
        );
    }
}
