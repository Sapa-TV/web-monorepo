use serde_json::from_value;
use serde_json::json;

use super::*;

fn vk_json() -> serde_json::Value {
    json!({
        "client_id": "client_id",
        "client_secret": "client_secret",
        "channel_url": "test_channel",
        "redirect_uri": "https://localhost/login-callback/vk-video-live",
        "credentials_redirect_uri": "https://localhost/creds-callback/vk-video-live",
        "csrf_ttl_secs": 600,
    })
}

#[test]
fn valid_config_is_accepted() {
    let config = from_value::<VkVideoLiveConfig>(vk_json()).expect("should deserialize");
    assert_eq!(config.csrf_ttl_secs, 600);
    assert_eq!(config.channel_url, "test_channel");
}

#[test]
fn missing_required_field_fails_validation() {
    let config = VkVideoLiveConfig::build(
        String::new(),
        "secret".to_string(),
        "test_channel".to_string(),
        "https://localhost/cb".to_string(),
        "https://localhost/creds/cb".to_string(),
        600,
    );
    assert!(matches!(
        config,
        Err(ConfigError::MissingField { field: "client_id" })
    ));
}

#[test]
fn empty_channel_url_fails_validation() {
    let config = VkVideoLiveConfig::build(
        "client_id".to_string(),
        "secret".to_string(),
        String::new(),
        "https://localhost/cb".to_string(),
        "https://localhost/creds/cb".to_string(),
        600,
    );
    assert!(matches!(
        config,
        Err(ConfigError::MissingField {
            field: "channel_url"
        })
    ));
}

#[test]
fn zero_ttl_is_rejected() {
    let value = vk_json();
    let config = VkVideoLiveConfig::build(
        value["client_id"].as_str().unwrap().to_string(),
        value["client_secret"].as_str().unwrap().to_string(),
        value["channel_url"].as_str().unwrap().to_string(),
        value["redirect_uri"].as_str().unwrap().to_string(),
        value["credentials_redirect_uri"]
            .as_str()
            .unwrap()
            .to_string(),
        0,
    );
    assert!(matches!(config, Err(ConfigError::InvalidCsrfTtl)));
}

#[test]
fn missing_csrf_ttl_defaults_to_600() {
    let mut value = vk_json();
    value
        .as_object_mut()
        .expect("object")
        .remove("csrf_ttl_secs");
    let config = from_value::<VkVideoLiveConfig>(value).expect("should deserialize");
    assert_eq!(config.csrf_ttl_secs, DEFAULT_CSRF_TTL_SECS);
}

#[test]
fn empty_required_field_fails_deserialization() {
    let mut value = vk_json();
    value["redirect_uri"] = json!("");
    assert!(from_value::<VkVideoLiveConfig>(value).is_err());
}
