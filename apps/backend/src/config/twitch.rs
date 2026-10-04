use serde::Deserialize;
use serde::de::Error as _;

use crate::error::config::ConfigError;

const DEFAULT_CSRF_TTL_SECS: u64 = 600;

fn default_csrf_ttl_secs() -> u64 {
    DEFAULT_CSRF_TTL_SECS
}

#[derive(Clone)]
#[non_exhaustive]
pub struct TwitchConfig {
    pub client_id: String,
    pub client_secret: String,
    pub broadcaster_id: String,
    pub redirect_uri: String,
    pub credentials_redirect_uri: String,
    pub csrf_ttl_secs: u64,
    _sealed: (),
}

impl TwitchConfig {
    pub fn build(
        client_id: String,
        client_secret: String,
        broadcaster_id: String,
        redirect_uri: String,
        credentials_redirect_uri: String,
        csrf_ttl_secs: u64,
    ) -> Result<Self, ConfigError> {
        let required = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("broadcaster_id", broadcaster_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            (
                "credentials_redirect_uri",
                credentials_redirect_uri.as_str(),
            ),
        ];
        for (name, value) in required {
            if value.is_empty() {
                return Err(ConfigError::MissingField { field: name });
            }
        }
        if csrf_ttl_secs == 0 {
            return Err(ConfigError::InvalidCsrfTtl);
        }
        Ok(Self {
            client_id,
            client_secret,
            broadcaster_id,
            redirect_uri,
            credentials_redirect_uri,
            csrf_ttl_secs,
            _sealed: (),
        })
    }
}

#[cfg(test)]
impl TwitchConfig {
    /// Canonical valid config for tests; tweak pub fields after clone if needed.
    pub(crate) fn fixture() -> Self {
        Self {
            client_id: "cid".to_string(),
            client_secret: "cs".to_string(),
            broadcaster_id: "bc".to_string(),
            redirect_uri: "https://localhost/cb".to_string(),
            credentials_redirect_uri: "https://localhost/creds/cb".to_string(),
            csrf_ttl_secs: 600,
            _sealed: (),
        }
    }
}

impl<'de> Deserialize<'de> for TwitchConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            client_id: String,
            client_secret: String,
            broadcaster_id: String,
            redirect_uri: String,
            credentials_redirect_uri: String,
            #[serde(default = "default_csrf_ttl_secs")]
            csrf_ttl_secs: u64,
        }

        let raw = Raw::deserialize(deserializer)?;
        TwitchConfig::build(
            raw.client_id,
            raw.client_secret,
            raw.broadcaster_id,
            raw.redirect_uri,
            raw.credentials_redirect_uri,
            raw.csrf_ttl_secs,
        )
        .map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
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
}
