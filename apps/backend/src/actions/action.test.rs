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
    let action = Action::new(
        ActionId::new(1),
        "reply".to_string(),
        ActionKind::ChatReply {
            message_template: "hi {username}".to_string(),
        },
        true,
        Utc::now(),
        Utc::now(),
    );
    assert_eq!(
        action.kind,
        ActionKind::ChatReply {
            message_template: "hi {username}".to_string()
        }
    );
}
