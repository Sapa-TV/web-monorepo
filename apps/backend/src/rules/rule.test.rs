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
    let contains = MessageConditions::new(MessageMatcher::StartsWith, Some("!spin".to_string()));
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
