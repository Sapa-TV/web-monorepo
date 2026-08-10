use std::sync::Arc;

use ::config::{Environment, File};
use serde::Deserialize;

use crate::config::twitch::TwitchConfig;

#[derive(Clone, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct Config {
    pub roulette_timeout_secs: u64,
    pub retention_secs: u64,
    pub queue_cleanup_interval_secs: u64,
    pub sessions_cleanup_interval_secs: u64,
    pub queue_default_limit: usize,
    pub port: u16,
    pub access_key: String,
    #[serde(deserialize_with = "deserialize_cors_origins")]
    pub cors_origins: Option<Vec<String>>,
    pub twitch: Option<Arc<TwitchConfig>>,
    pub admin_twitch_id: Option<String>,
    pub session_ttl_secs: u64,
    pub cookie_secure: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roulette_timeout_secs: 10,
            retention_secs: 24 * 60 * 60,
            queue_cleanup_interval_secs: 60 * 60,
            sessions_cleanup_interval_secs: 60 * 60,
            queue_default_limit: 20,
            port: 3000,
            access_key: String::new(),
            cors_origins: None,
            twitch: None,
            admin_twitch_id: None,
            session_ttl_secs: 24 * 60 * 60,
            cookie_secure: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if let Err(e) = dotenvy::dotenv() {
            tracing::warn!("failed to load .env: {e}");
        }

        let settings = ::config::Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::default())
            .build()
            .expect("failed to load configuration");
        let config: Config = settings.try_deserialize().expect("invalid configuration");

        if config.access_key.is_empty() {
            panic!("ACCESS_KEY must be set (via ACCESS_KEY env or config.toml)");
        }
        config
    }
}

fn deserialize_cors_origins<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        List(Vec<String>),
        Comma(String),
        Null,
    }

    let trimmed: Vec<String> = match Raw::deserialize(deserializer)? {
        Raw::List(list) => list
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Raw::Comma(s) => s
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect(),
        Raw::Null => return Ok(None),
    };
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

#[cfg(test)]
impl Config {
    pub fn test_config() -> Self {
        Self {
            roulette_timeout_secs: 0,
            retention_secs: 24 * 60 * 60,
            queue_cleanup_interval_secs: 60 * 60,
            sessions_cleanup_interval_secs: 60 * 60,
            queue_default_limit: 20,
            port: 3000,
            access_key: "test-key".to_string(),
            cors_origins: None,
            twitch: None,
            admin_twitch_id: None,
            session_ttl_secs: 24 * 60 * 60,
            cookie_secure: false,
        }
    }
}
