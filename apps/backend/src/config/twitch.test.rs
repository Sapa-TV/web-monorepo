use serde_json::from_value;
use serde_json::json;

use super::*;

fn twitch_json() -> serde_json::Value {
    json!({
        "client_id": "client_id",
        "client_secret": "client_secret",
        "broadcaster_id": "broadcaster_id",
        "redirect_uri": "https://localhost/callback",
        "credentials_redirect_uri": "https://localhost/creds/callback",
        "csrf_ttl_secs": 600,
    })
}

#[test]
fn valid_config_is_accepted() {
    let config = from_value::<TwitchConfig>(twitch_json()).expect("should deserialize");
    assert_eq!(config.csrf_ttl_secs, 600);
    assert_eq!(
        config.credentials_redirect_uri,
        "https://localhost/creds/callback"
    );
}

#[test]
fn missing_required_field_fails_validation() {
    let config = TwitchConfig::build(
        String::new(),
        "secret".to_string(),
        "broadcaster".to_string(),
        "https://localhost/callback".to_string(),
        "https://localhost/creds/callback".to_string(),
        600,
    );
    assert!(matches!(
        config,
        Err(ConfigError::MissingField { field: "client_id" })
    ));
}

#[test]
fn empty_credentials_redirect_uri_fails_validation() {
    let config = TwitchConfig::build(
        "client_id".to_string(),
        "secret".to_string(),
        "broadcaster".to_string(),
        "https://localhost/callback".to_string(),
        String::new(),
        600,
    );
    assert!(matches!(
        config,
        Err(ConfigError::MissingField {
            field: "credentials_redirect_uri"
        })
    ));
}

#[test]
fn zero_ttl_is_rejected() {
    let value = twitch_json();
    let config = TwitchConfig::build(
        value["client_id"].as_str().unwrap().to_string(),
        value["client_secret"].as_str().unwrap().to_string(),
        value["broadcaster_id"].as_str().unwrap().to_string(),
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
    let mut value = twitch_json();
    value
        .as_object_mut()
        .expect("object")
        .remove("csrf_ttl_secs");
    let config = from_value::<TwitchConfig>(value).expect("should deserialize");
    assert_eq!(config.csrf_ttl_secs, DEFAULT_CSRF_TTL_SECS);
}

#[test]
fn empty_required_field_fails_deserialization() {
    let mut value = twitch_json();
    value["redirect_uri"] = json!("");
    assert!(from_value::<TwitchConfig>(value).is_err());
}
