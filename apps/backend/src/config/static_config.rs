use std::sync::Arc;

use ::config::{Environment, File};
use serde::Deserialize;

use crate::config::runtime::RuntimeConfig;
use crate::config::twitch::TwitchConfig;

#[derive(Clone, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct StaticConfig {
    pub port: u16,
    #[serde(deserialize_with = "deserialize_cors_origins")]
    pub cors_origins: Option<Vec<String>>,
    pub cookie_secure: bool,
    pub twitch: Option<Arc<TwitchConfig>>,
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            cors_origins: None,
            cookie_secure: false,
            twitch: None,
        }
    }
}

impl StaticConfig {
    pub fn load() -> (Self, Option<RuntimeConfig>) {
        if let Err(e) = dotenvy::dotenv()
            && !e.not_found()
        {
            tracing::warn!("failed to load .env: {e}");
        }

        let settings = ::config::Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::default().separator("__"))
            .build()
            .expect("failed to load configuration");
        let raw: RawConfig = settings.try_deserialize().expect("invalid configuration");
        Self::split(raw)
    }

    fn split(raw: RawConfig) -> (Self, Option<RuntimeConfig>) {
        let static_cfg = Self {
            port: raw.port,
            cors_origins: raw.cors_origins,
            cookie_secure: raw.cookie_secure,
            twitch: raw.twitch,
        };
        let seed = RuntimeConfig {
            access_key: String::new(),
            roulette_timeout_secs: raw.roulette_timeout_secs,
            retention_secs: raw.retention_secs,
            queue_cleanup_interval_secs: raw.queue_cleanup_interval_secs,
            sessions_cleanup_interval_secs: raw.sessions_cleanup_interval_secs,
            queue_default_limit: raw.queue_default_limit,
            session_ttl_secs: raw.session_ttl_secs,
        };
        (static_cfg, Some(seed))
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawConfig {
    roulette_timeout_secs: u64,
    retention_secs: u64,
    queue_cleanup_interval_secs: u64,
    sessions_cleanup_interval_secs: u64,
    queue_default_limit: usize,
    port: u16,
    #[serde(deserialize_with = "deserialize_cors_origins")]
    cors_origins: Option<Vec<String>>,
    twitch: Option<Arc<TwitchConfig>>,
    session_ttl_secs: u64,
    cookie_secure: bool,
}

impl Default for RawConfig {
    fn default() -> Self {
        let runtime = RuntimeConfig::default();
        let static_cfg = StaticConfig::default();
        Self {
            roulette_timeout_secs: runtime.roulette_timeout_secs,
            retention_secs: runtime.retention_secs,
            queue_cleanup_interval_secs: runtime.queue_cleanup_interval_secs,
            sessions_cleanup_interval_secs: runtime.sessions_cleanup_interval_secs,
            queue_default_limit: runtime.queue_default_limit,
            session_ttl_secs: runtime.session_ttl_secs,
            port: static_cfg.port,
            cors_origins: static_cfg.cors_origins,
            twitch: static_cfg.twitch,
            cookie_secure: static_cfg.cookie_secure,
        }
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
impl StaticConfig {
    pub fn test_config() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_separates_static_and_runtime() {
        let raw: RawConfig = serde_json::from_str(
            r#"{
                "port": 4321,
                "cors_origins": ["https://a.com", "https://b.com"],
                "cookie_secure": true,
                "twitch": {
                    "client_id": "cid",
                    "client_secret": "cs",
                    "broadcaster_id": "bc",
                    "redirect_uri": "https://localhost/cb",
                    "credentials_redirect_uri": "https://localhost/creds/cb",
                    "csrf_ttl_secs": 600
                },
                "roulette_timeout_secs": 30,
                "session_ttl_secs": 3600
            }"#,
        )
        .unwrap();

        let (static_cfg, seed) = StaticConfig::split(raw);

        assert_eq!(static_cfg.port, 4321);
        assert_eq!(
            static_cfg.cors_origins.as_deref().unwrap(),
            &["https://a.com", "https://b.com"]
        );
        assert!(static_cfg.cookie_secure);
        assert_eq!(static_cfg.twitch.as_deref().unwrap().broadcaster_id, "bc");

        let seed = seed.expect("seed present");
        assert_eq!(seed.roulette_timeout_secs, 30);
        assert_eq!(seed.session_ttl_secs, 3600);
        assert_eq!(seed.retention_secs, RuntimeConfig::default().retention_secs);
        assert!(seed.access_key.is_empty());
    }

    #[test]
    fn cors_origins_accepts_comma_string() {
        let raw: RawConfig =
            serde_json::from_str(r#"{ "cors_origins": " https://a.com , https://b.com ,, " }"#)
                .unwrap();

        let (static_cfg, _) = StaticConfig::split(raw);

        assert_eq!(
            static_cfg.cors_origins.as_deref().unwrap(),
            &["https://a.com", "https://b.com"]
        );
    }

    #[test]
    fn split_passes_twitch_through() {
        let raw: RawConfig = serde_json::from_str(
            r#"{
                "twitch": {
                    "client_id": "cid",
                    "client_secret": "cs",
                    "broadcaster_id": "bc",
                    "redirect_uri": "https://localhost/cb",
                    "credentials_redirect_uri": "https://localhost/creds/cb",
                    "csrf_ttl_secs": 600
                }
            }"#,
        )
        .unwrap();

        let (static_cfg, _) = StaticConfig::split(raw);

        let twitch = static_cfg.twitch.expect("twitch set");
        assert_eq!(twitch.client_id, "cid");
    }
}
