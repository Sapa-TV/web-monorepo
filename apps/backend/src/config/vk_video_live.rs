use serde::Deserialize;
use serde::de::Error as _;

use crate::error::config::ConfigError;

#[derive(Clone)]
#[non_exhaustive]
pub struct VkVideoLiveConfig {
    pub client_id: String,
    pub client_secret: String,
    pub channel_url: String,
    pub redirect_uri: String,
    pub credentials_redirect_uri: String,
    pub csrf_ttl_secs: u64,
    _sealed: (),
}

impl VkVideoLiveConfig {
    pub fn build(
        client_id: String,
        client_secret: String,
        channel_url: String,
        redirect_uri: String,
        credentials_redirect_uri: String,
        csrf_ttl_secs: u64,
    ) -> Result<Self, ConfigError> {
        let required = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("channel_url", channel_url.as_str()),
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
            channel_url,
            redirect_uri,
            credentials_redirect_uri,
            csrf_ttl_secs,
            _sealed: (),
        })
    }
}

#[cfg(test)]
impl VkVideoLiveConfig {
    /// Canonical valid config for tests; tweak pub fields after clone if needed.
    pub(crate) fn fixture() -> Self {
        Self {
            client_id: "cid".to_string(),
            client_secret: "cs".to_string(),
            channel_url: "sapushka_".to_string(),
            redirect_uri: "https://localhost/login-callback/vk-video-live".to_string(),
            credentials_redirect_uri: "https://localhost/creds-callback/vk-video-live".to_string(),
            csrf_ttl_secs: 600,
            _sealed: (),
        }
    }
}

impl<'de> Deserialize<'de> for VkVideoLiveConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            client_id: String,
            client_secret: String,
            channel_url: String,
            redirect_uri: String,
            credentials_redirect_uri: String,
            csrf_ttl_secs: u64,
        }

        let raw = Raw::deserialize(deserializer)?;
        VkVideoLiveConfig::build(
            raw.client_id,
            raw.client_secret,
            raw.channel_url,
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

    fn vk_json() -> serde_json::Value {
        json!({
            "client_id": "client_id",
            "client_secret": "client_secret",
            "channel_url": "sapushka_",
            "redirect_uri": "https://localhost/login-callback/vk-video-live",
            "credentials_redirect_uri": "https://localhost/creds-callback/vk-video-live",
            "csrf_ttl_secs": 600,
        })
    }

    #[test]
    fn valid_config_is_accepted() {
        let config = from_value::<VkVideoLiveConfig>(vk_json()).expect("should deserialize");
        assert_eq!(config.csrf_ttl_secs, 600);
        assert_eq!(config.channel_url, "sapushka_");
    }

    #[test]
    fn missing_required_field_fails_validation() {
        let config = VkVideoLiveConfig::build(
            String::new(),
            "secret".to_string(),
            "sapushka_".to_string(),
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
    fn missing_csrf_ttl_fails_deserialization() {
        let mut value = vk_json();
        value
            .as_object_mut()
            .expect("object")
            .remove("csrf_ttl_secs");
        assert!(from_value::<VkVideoLiveConfig>(value).is_err());
    }

    #[test]
    fn empty_required_field_fails_deserialization() {
        let mut value = vk_json();
        value["redirect_uri"] = json!("");
        assert!(from_value::<VkVideoLiveConfig>(value).is_err());
    }
}
