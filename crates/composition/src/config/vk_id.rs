use serde::Deserialize;
use serde::de::Error as _;

use super::{default_csrf_ttl_secs, validate_csrf_ttl, validate_required};
use model::config::ConfigError;

const DEFAULT_API_BASE_URL: &str = "https://id.vk.com";

#[derive(Clone)]
#[non_exhaustive]
pub struct VkIdConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub api_base_url: String,
    pub csrf_ttl_secs: u64,
    _sealed: (),
}

impl VkIdConfig {
    pub fn build(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        api_base_url: String,
        csrf_ttl_secs: u64,
    ) -> Result<Self, ConfigError> {
        validate_required(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("api_base_url", api_base_url.as_str()),
        ])?;
        validate_csrf_ttl(csrf_ttl_secs)?;
        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
            api_base_url,
            csrf_ttl_secs,
            _sealed: (),
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
impl VkIdConfig {
    pub fn fixture() -> Self {
        Self {
            client_id: "vk-client".to_string(),
            client_secret: "vk-secret".to_string(),
            redirect_uri: "https://localhost/auth/vk/callback".to_string(),
            api_base_url: DEFAULT_API_BASE_URL.to_string(),
            csrf_ttl_secs: 600,
            _sealed: (),
        }
    }
}

impl<'de> Deserialize<'de> for VkIdConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            client_id: String,
            client_secret: String,
            redirect_uri: String,
            #[serde(default = "default_api_base_url")]
            api_base_url: String,
            #[serde(default = "default_csrf_ttl_secs")]
            csrf_ttl_secs: u64,
        }

        let raw = Raw::deserialize(deserializer)?;
        VkIdConfig::build(
            raw.client_id,
            raw.client_secret,
            raw.redirect_uri,
            raw.api_base_url,
            raw.csrf_ttl_secs,
        )
        .map_err(D::Error::custom)
    }
}

fn default_api_base_url() -> String {
    DEFAULT_API_BASE_URL.to_string()
}

#[cfg(test)]
#[path = "vk_id.test.rs"]
mod tests;
