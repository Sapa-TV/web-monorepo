use serde::{Deserialize, Serialize};

use crate::consts::queue;
use crate::consts::roulette;
use crate::consts::session;
use crate::error::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct QueueRuntimeConfig {
    pub default_limit: usize,
    pub retention_secs: u64,
    pub cleanup_interval_secs: u64,
}

impl Default for QueueRuntimeConfig {
    fn default() -> Self {
        Self {
            default_limit: queue::DEFAULT_LIMIT,
            retention_secs: queue::RETENTION_SECS,
            cleanup_interval_secs: queue::CLEANUP_INTERVAL_SECS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SessionRuntimeConfig {
    pub ttl_secs: u64,
    pub cleanup_interval_secs: u64,
}

impl Default for SessionRuntimeConfig {
    fn default() -> Self {
        Self {
            ttl_secs: session::TTL_SECS,
            cleanup_interval_secs: session::CLEANUP_INTERVAL_SECS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct RouletteRuntimeConfig {
    pub timeout_secs: u64,
}

impl Default for RouletteRuntimeConfig {
    fn default() -> Self {
        Self {
            timeout_secs: roulette::TIMEOUT_SECS,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct RuntimeConfig {
    pub widget_access_key: String,
    pub queue: QueueRuntimeConfig,
    pub session: SessionRuntimeConfig,
    pub roulette: RouletteRuntimeConfig,
}

impl RuntimeConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.widget_access_key.is_empty() {
            return Err(ConfigError::InvalidWidgetAccessKey);
        }
        for (field, value) in [
            ("queue.default_limit", self.queue.default_limit as u64),
            ("queue.retention_secs", self.queue.retention_secs),
            (
                "queue.cleanup_interval_secs",
                self.queue.cleanup_interval_secs,
            ),
            ("session.ttl_secs", self.session.ttl_secs),
            (
                "session.cleanup_interval_secs",
                self.session.cleanup_interval_secs,
            ),
            ("roulette.timeout_secs", self.roulette.timeout_secs),
        ] {
            if value == 0 {
                return Err(ConfigError::InvalidValue { field });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
impl RuntimeConfig {
    pub fn test_runtime(widget_access_key: &str) -> Self {
        Self {
            widget_access_key: widget_access_key.to_string(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_empty_widget_access_key() {
        let cfg = RuntimeConfig::default();
        assert!(cfg.widget_access_key.is_empty());
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::InvalidWidgetAccessKey)
        ));
    }

    #[test]
    fn valid_config_accepted() {
        assert!(RuntimeConfig::test_runtime("secret").validate().is_ok());
    }

    #[test]
    fn zero_values_rejected() {
        let base = RuntimeConfig::test_runtime("secret");
        for (field, value) in [
            ("roulette.timeout_secs", 0_u64),
            ("queue.retention_secs", 0_u64),
            ("queue.cleanup_interval_secs", 0_u64),
            ("session.cleanup_interval_secs", 0_u64),
            ("session.ttl_secs", 0_u64),
        ] {
            let mut cfg = base.clone();
            apply_u64(&mut cfg, field, value);
            assert!(
                matches!(
                    cfg.validate(),
                    Err(ConfigError::InvalidValue { field: actual }) if actual == field
                ),
                "expected InvalidValue for {field}"
            );
        }

        let mut cfg = base.clone();
        cfg.queue.default_limit = 0;
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::InvalidValue {
                field: "queue.default_limit"
            })
        ));
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = RuntimeConfig::test_runtime("secret");
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: RuntimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, cfg);
    }

    #[test]
    fn serde_missing_sections_use_defaults() {
        let json = r#"{"widget_access_key":"secret"}"#;
        let cfg: RuntimeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.widget_access_key, "secret");
        assert_eq!(cfg.queue, QueueRuntimeConfig::default());
        assert_eq!(cfg.session, SessionRuntimeConfig::default());
        assert_eq!(cfg.roulette, RouletteRuntimeConfig::default());
    }

    #[test]
    fn serde_unknown_field_ignored() {
        let json = r#"{"widget_access_key":"secret","unknown_field":42}"#;
        let cfg: RuntimeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.widget_access_key, "secret");
    }

    fn apply_u64(cfg: &mut RuntimeConfig, field: &str, value: u64) {
        match field {
            "roulette.timeout_secs" => cfg.roulette.timeout_secs = value,
            "queue.retention_secs" => cfg.queue.retention_secs = value,
            "queue.cleanup_interval_secs" => cfg.queue.cleanup_interval_secs = value,
            "session.cleanup_interval_secs" => cfg.session.cleanup_interval_secs = value,
            "session.ttl_secs" => cfg.session.ttl_secs = value,
            _ => unreachable!(),
        }
    }
}
