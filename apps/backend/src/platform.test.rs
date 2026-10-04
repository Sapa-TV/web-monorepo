use super::*;

#[test]
fn platform_serde_roundtrip() {
    let platform = Platform::from_id(PlatformId::YOUTUBE);
    let json = serde_json::to_value(&platform).unwrap();
    assert_eq!(json, serde_json::json!({"id": 2, "name": "youtube"}));
    let back: Platform = serde_json::from_value(json).unwrap();
    assert_eq!(back, platform);
}
