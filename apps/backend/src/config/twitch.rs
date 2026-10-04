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
#[path = "twitch.test.rs"]
mod tests;
