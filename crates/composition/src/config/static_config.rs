use std::sync::Arc;

use ::config::{Environment, File};
use serde::Deserialize;

use crate::config::twitch::TwitchConfig;
use crate::config::vk_video_live::VkVideoLiveConfig;
use model::config::runtime::{
    QueueRuntimeConfig, RouletteRuntimeConfig, RuntimeConfig, SessionRuntimeConfig,
    SheetsRuntimeConfig,
};
use model::consts::{server, session};

#[derive(Clone, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct StaticConfig {
    pub port: u16,
    #[serde(deserialize_with = "deserialize_comma_string_list")]
    pub cors_origins: Option<Vec<String>>,
    pub cookie_secure: bool,
    pub guest_session_ttl_secs: u64,
    pub twitch: Option<Arc<TwitchConfig>>,
    pub vk_video_live: Option<Arc<VkVideoLiveConfig>>,
    pub google_service_account_key_base64: Option<String>,
    #[serde(deserialize_with = "deserialize_comma_string_list")]
    pub admin_twitch_ids: Option<Vec<String>>,
    pub srs_hook_secret: Option<String>,
    pub srs_stream_key: Option<String>,
    pub srs_api_url: Option<String>,
    pub site_url: String,
    pub vapid_private_key: Option<String>,
    pub vapid_public_key: Option<String>,
    pub vapid_subject: String,
    #[serde(skip)]
    _sealed: (),
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            port: server::PORT,
            cors_origins: None,
            cookie_secure: true,
            guest_session_ttl_secs: session::GUEST_TTL_SECS,
            twitch: None,
            vk_video_live: None,
            google_service_account_key_base64: None,
            admin_twitch_ids: None,
            srs_hook_secret: None,
            srs_stream_key: None,
            srs_api_url: None,
            site_url: "https://sapa-tv.ru".to_string(),
            vapid_private_key: None,
            vapid_public_key: None,
            vapid_subject: "mailto:admin@sapa-tv.ru".to_string(),
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
        admin_twitch_ids: Option<Vec<String>>,
    ) -> Self {
        Self {
            port,
            cors_origins,
            cookie_secure,
            guest_session_ttl_secs: session::GUEST_TTL_SECS,
            twitch,
            vk_video_live,
            google_service_account_key_base64,
            admin_twitch_ids,
            srs_hook_secret: None,
            srs_stream_key: None,
            srs_api_url: None,
            site_url: "https://sapa-tv.ru".to_string(),
            vapid_private_key: None,
            vapid_public_key: None,
            vapid_subject: "mailto:admin@sapa-tv.ru".to_string(),
            _sealed: (),
        }
    }

    pub fn with_srs(mut self, hook_secret: Option<String>, api_url: Option<String>) -> Self {
        self.srs_hook_secret = hook_secret;
        self.srs_api_url = api_url;
        self
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
            guest_session_ttl_secs: raw.guest_session_ttl_secs,
            twitch: raw.twitch,
            vk_video_live: raw.vk_video_live,
            google_service_account_key_base64: raw.google_service_account_key_base64,
            admin_twitch_ids: raw.admin_twitch_ids,
            srs_hook_secret: raw.srs_hook_secret,
            srs_stream_key: raw.srs_stream_key,
            srs_api_url: raw.srs_api_url,
            site_url: raw.site_url,
            vapid_private_key: raw.vapid_private_key,
            vapid_public_key: raw.vapid_public_key,
            vapid_subject: raw.vapid_subject,
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
    #[serde(deserialize_with = "deserialize_comma_string_list")]
    cors_origins: Option<Vec<String>>,
    twitch: Option<Arc<TwitchConfig>>,
    vk_video_live: Option<Arc<VkVideoLiveConfig>>,
    session_ttl_secs: u64,
    guest_session_ttl_secs: u64,
    cookie_secure: bool,
    google_service_account_key_base64: Option<String>,
    google_spreadsheet_id: String,
    #[serde(deserialize_with = "deserialize_comma_string_list")]
    admin_twitch_ids: Option<Vec<String>>,
    srs_hook_secret: Option<String>,
    srs_stream_key: Option<String>,
    srs_api_url: Option<String>,
    site_url: String,
    vapid_private_key: Option<String>,
    vapid_public_key: Option<String>,
    vapid_subject: String,
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
            guest_session_ttl_secs: static_cfg.guest_session_ttl_secs,
            port: static_cfg.port,
            cors_origins: static_cfg.cors_origins,
            twitch: static_cfg.twitch,
            vk_video_live: static_cfg.vk_video_live,
            cookie_secure: static_cfg.cookie_secure,
            google_service_account_key_base64: None,
            google_spreadsheet_id: String::new(),
            admin_twitch_ids: None,
            srs_hook_secret: None,
            srs_stream_key: None,
            srs_api_url: None,
            site_url: static_cfg.site_url,
            vapid_private_key: static_cfg.vapid_private_key,
            vapid_public_key: static_cfg.vapid_public_key,
            vapid_subject: static_cfg.vapid_subject,
        }
    }
}

fn deserialize_comma_string_list<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
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

#[cfg(any(test, feature = "test-support"))]
impl StaticConfig {
    pub fn test_config() -> Self {
        Self::default()
    }

    pub fn with_twitch(twitch: Option<Arc<TwitchConfig>>) -> Self {
        Self {
            twitch,
            ..Self::default()
        }
    }

    pub fn with_vk_video_live(vk: Option<Arc<VkVideoLiveConfig>>) -> Self {
        Self {
            vk_video_live: vk,
            ..Self::default()
        }
    }

    pub fn with_srs_hook_secret(secret: Option<String>) -> Self {
        Self {
            srs_hook_secret: secret,
            ..Self::default()
        }
    }

    pub fn with_srs_stream_key(mut self, key: Option<String>) -> Self {
        self.srs_stream_key = key;
        self
    }
}

#[cfg(test)]
#[path = "static_config.test.rs"]
mod tests;
