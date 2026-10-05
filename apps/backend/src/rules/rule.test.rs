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
        RuleConditions::RewardRedemption(RewardConditions::new(vec!["reward-1".to_string()], None));

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
    let contains = MessageConditions::new(MessageMatcher::StartsWith, Some("!spin".to_string()));
    let json = serde_json::to_value(contains).unwrap();
    assert_eq!(json["matcher"], "starts_with");
}

#[test]
fn reward_conditions_serialize_with_trigger_tag() {
    let conditions = RuleConditions::RewardRedemption(RewardConditions::new(Vec::new(), None));
    let json = serde_json::to_value(conditions).unwrap();
    assert_eq!(json["trigger"], "reward_redemption");
    assert_eq!(json["reward_ids"], serde_json::json!([]));
}

#[test]
fn reward_conditions_deserialize_legacy_reward_id() {
    let legacy = serde_json::json!({"trigger": "reward_redemption", "reward_id": "reward-1"});
    let RuleConditions::RewardRedemption(conditions) =
        serde_json::from_value::<RuleConditions>(legacy).unwrap()
    else {
        panic!("expected reward conditions");
    };
    assert_eq!(conditions.reward_ids, vec!["reward-1".to_string()]);
    assert_eq!(conditions.platform, None);
}

#[test]
fn reward_conditions_deserialize_merges_legacy_and_list() {
    let mixed = serde_json::json!({
        "trigger": "reward_redemption",
        "reward_id": "reward-1",
        "reward_ids": ["reward-2"],
        "platform": 3
    });
    let RuleConditions::RewardRedemption(conditions) =
        serde_json::from_value::<RuleConditions>(mixed).unwrap()
    else {
        panic!("expected reward conditions");
    };
    assert_eq!(
        conditions.reward_ids,
        vec!["reward-2".to_string(), "reward-1".to_string()]
    );
    assert_eq!(conditions.platform, Some(PlatformId::VK_VIDEO_LIVE));
}
