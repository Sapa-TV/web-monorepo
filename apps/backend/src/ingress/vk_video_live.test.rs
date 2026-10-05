use super::*;
use crate::ingress::event::PlatformEventPayload;
use vk_video_live::events::{PushEvent, RewardDemandEvent};

fn chat(id: u64, nick: &str, text: &str) -> PushEvent {
    PushEvent::ChatMessage(ChatMessageEvent::new(
        id,
        555,
        nick,
        1_787_682_950,
        text,
        false,
        false,
    ))
}

#[test]
fn normal_message_maps_to_platform_event() {
    let mapped =
        push_event_from(PlatformId::VK_VIDEO_LIVE, chat(651, "tester", "hi")).expect("mapped");
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
    let PushEvent::ChatMessage(mut e) = chat(1, "n", "t") else {
        panic!("expected chat");
    };
    e.is_deleted = true;
    assert!(push_event_from(PlatformId::VK_VIDEO_LIVE, PushEvent::ChatMessage(e)).is_none());
}

#[test]
fn private_message_is_skipped() {
    let PushEvent::ChatMessage(mut e) = chat(1, "n", "t") else {
        panic!("expected chat");
    };
    e.is_private = true;
    assert!(push_event_from(PlatformId::VK_VIDEO_LIVE, PushEvent::ChatMessage(e)).is_none());
}

#[test]
fn reward_demand_maps_to_redemption() {
    let demand = PushEvent::RewardDemand(RewardDemandEvent::new(
        9179,
        555,
        "tester",
        "bc211809-1e52-4180-8dbc-e106649ef78a",
        "pending",
        1_787_682_950,
    ));
    let mapped = push_event_from(PlatformId::VK_VIDEO_LIVE, demand).expect("mapped");
    assert_eq!(mapped.event_id, "9179");
    match &mapped.payload {
        PlatformEventPayload::RewardRedemption(r) => {
            assert_eq!(r.user_id, "555");
            assert_eq!(r.user_name, "tester");
            assert_eq!(r.reward_id, "bc211809-1e52-4180-8dbc-e106649ef78a");
            assert_eq!(r.status, "pending");
        }
        other => panic!("expected reward redemption payload, got {other:?}"),
    }
}
