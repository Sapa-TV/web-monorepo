use std::sync::Arc;

use ::config::{Environment, File};
use serde::Deserialize;

use crate::config::runtime::{
    QueueRuntimeConfig, RouletteRuntimeConfig, RuntimeConfig, SessionRuntimeConfig,
    SheetsRuntimeConfig,
};
use crate::config::twitch::TwitchConfig;
use crate::config::vk_video_live::VkVideoLiveConfig;
use crate::consts::server;

#[derive(Clone, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct StaticConfig {
    pub port: u16,
    #[serde(deserialize_with = "deserialize_cors_origins")]
    pub cors_origins: Option<Vec<String>>,
    pub cookie_secure: bool,
    pub twitch: Option<Arc<TwitchConfig>>,
    pub vk_video_live: Option<Arc<VkVideoLiveConfig>>,
    pub google_service_account_key_base64: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            port: server::PORT,
            cors_origins: None,
            cookie_secure: false,
            twitch: None,
            vk_video_live: None,
            google_service_account_key_base64: None,
            _sealed: (),
        }
    }
}

impl StaticConfig {
    pub fn new(
        port: u16,
        cors_origins: Option<Vec<String>>,
        cookie_secure: bool,
        twitch: Option<Arc<TwitchConfig>>,
        vk_video_live: Option<Arc<VkVideoLiveConfig>>,
        google_service_account_key_base64: Option<String>,
    ) -> Self {
        Self {
            port,
            cors_origins,
            cookie_secure,
            twitch,
            vk_video_live,
            google_service_account_key_base64,
            _sealed: (),
        }
    }

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
            vk_video_live: raw.vk_video_live,
            google_service_account_key_base64: raw.google_service_account_key_base64,
            _sealed: (),
        };
        let seed = RuntimeConfig::new(
            String::new(),
            QueueRuntimeConfig::new(
                raw.queue_default_limit,
                raw.retention_secs,
                raw.queue_cleanup_interval_secs,
            ),
            SessionRuntimeConfig::new(raw.session_ttl_secs, raw.sessions_cleanup_interval_secs),
            RouletteRuntimeConfig::new(raw.roulette_timeout_secs),
            SheetsRuntimeConfig::new(raw.google_spreadsheet_id),
        );
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
    vk_video_live: Option<Arc<VkVideoLiveConfig>>,
    session_ttl_secs: u64,
    cookie_secure: bool,
    google_service_account_key_base64: Option<String>,
    google_spreadsheet_id: String,
}

impl Default for RawConfig {
    fn default() -> Self {
        let runtime = RuntimeConfig::default();
        let static_cfg = StaticConfig::default();
        Self {
            roulette_timeout_secs: runtime.roulette.timeout_secs,
            retention_secs: runtime.queue.retention_secs,
            queue_cleanup_interval_secs: runtime.queue.cleanup_interval_secs,
            sessions_cleanup_interval_secs: runtime.session.cleanup_interval_secs,
            queue_default_limit: runtime.queue.default_limit,
            session_ttl_secs: runtime.session.ttl_secs,
            port: static_cfg.port,
            cors_origins: static_cfg.cors_origins,
            twitch: static_cfg.twitch,
            vk_video_live: static_cfg.vk_video_live,
            cookie_secure: static_cfg.cookie_secure,
            google_service_account_key_base64: None,
            google_spreadsheet_id: String::new(),
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

    pub(crate) fn with_twitch(twitch: Option<Arc<TwitchConfig>>) -> Self {
        Self {
            twitch,
            ..Self::default()
        }
    }

    pub(crate) fn with_vk_video_live(vk: Option<Arc<VkVideoLiveConfig>>) -> Self {
        Self {
            vk_video_live: vk,
            ..Self::default()
        }
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
        assert_eq!(seed.roulette.timeout_secs, 30);
        assert_eq!(seed.session.ttl_secs, 3600);
        assert_eq!(
            seed.queue.retention_secs,
            QueueRuntimeConfig::default().retention_secs
        );
        assert!(seed.widget_access_key.is_empty());
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
        assert!(static_cfg.vk_video_live.is_none());
    }

    #[test]
    fn split_passes_vk_video_live_through() {
        let raw: RawConfig = serde_json::from_str(
            r#"{
                "vk_video_live": {
                    "client_id": "vk-cid",
                    "client_secret": "vk-cs",
                    "channel_url": "test_channel",
                    "redirect_uri": "https://localhost/login-callback/vk-video-live",
                    "credentials_redirect_uri": "https://localhost/creds-callback/vk-video-live",
                    "csrf_ttl_secs": 600
                }
            }"#,
        )
        .unwrap();

        let (static_cfg, _) = StaticConfig::split(raw);

        let vk = static_cfg.vk_video_live.expect("vk set");
        assert_eq!(vk.client_id, "vk-cid");
        assert_eq!(vk.channel_url, "test_channel");
        assert!(static_cfg.twitch.is_none());
    }
}
