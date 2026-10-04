use super::*;
use crate::ingress::event::PlatformEventPayload;

fn event(id: u64, nick: &str, text: &str) -> ChatMessageEvent {
    ChatMessageEvent::new(id, 555, nick, 1_787_682_950, text, false, false)
}

#[test]
fn normal_message_maps_to_platform_event() {
    let mapped =
        chat_event_from(PlatformId::VK_VIDEO_LIVE, &event(651, "tester", "hi")).expect("mapped");
    assert_eq!(mapped.platform, PlatformId::VK_VIDEO_LIVE);
    assert_eq!(mapped.event_id, "651");
    match &mapped.payload {
        PlatformEventPayload::ChatMessage(msg) => {
            assert_eq!(msg.user_id, "555");
            assert_eq!(msg.user_name, "tester");
            assert_eq!(msg.text, "hi");
        }
        other => panic!("expected chat message payload, got {other:?}"),
    }
}

#[test]
fn deleted_message_is_skipped() {
    let mut e = event(1, "n", "t");
    e.is_deleted = true;
    assert!(chat_event_from(PlatformId::VK_VIDEO_LIVE, &e).is_none());
}

#[test]
fn private_message_is_skipped() {
    let mut e = event(1, "n", "t");
    e.is_private = true;
    assert!(chat_event_from(PlatformId::VK_VIDEO_LIVE, &e).is_none());
}
