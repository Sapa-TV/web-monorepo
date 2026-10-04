use strum::IntoDiscriminant;

use super::*;

#[test]
fn chat_message_payload_type_name() {
    let event = PlatformEvent::chat_message(
        PlatformId::TWITCH,
        "msg-1",
        "1".to_string(),
        "viewer".to_string(),
        "hello".to_string(),
    );
    assert_eq!(event.payload.type_name(), "chat_message");
    assert_eq!(event.event_id, "msg-1");
    match &event.payload {
        PlatformEventPayload::ChatMessage(msg) => assert_eq!(msg.text, "hello"),
        PlatformEventPayload::RewardRedemption(_) => unreachable!(),
    }
}

#[test]
fn reward_redemption_payload_type_name() {
    let event = PlatformEvent::reward_redemption(
        PlatformId::TWITCH,
        "red-1",
        "1".to_string(),
        "viewer".to_string(),
        "reward-9".to_string(),
        "Spin".to_string(),
        500,
        "please".to_string(),
        "unfulfilled".to_string(),
    );
    assert_eq!(event.payload.type_name(), "reward_redemption");
    assert_eq!(event.event_id, "red-1");
    match &event.payload {
        PlatformEventPayload::ChatMessage(_) => unreachable!(),
        PlatformEventPayload::RewardRedemption(red) => {
            assert_eq!(red.reward_title, "Spin");
            assert_eq!(red.reward_cost, 500);
            assert_eq!(red.status, "unfulfilled");
        }
    }
}

#[test]
fn payload_discriminant_maps_to_trigger() {
    let event = PlatformEvent::chat_message(
        PlatformId::TWITCH,
        "msg-1",
        "1".to_string(),
        "viewer".to_string(),
        "hello".to_string(),
    );
    assert_eq!(event.payload.discriminant(), RuleTrigger::ChatMessage);

    let event = PlatformEvent::reward_redemption(
        PlatformId::TWITCH,
        "red-1",
        "1".to_string(),
        "viewer".to_string(),
        "reward-9".to_string(),
        "Spin".to_string(),
        500,
        "".to_string(),
        "unfulfilled".to_string(),
    );
    assert_eq!(event.payload.discriminant(), RuleTrigger::RewardRedemption);
}
