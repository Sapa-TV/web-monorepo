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
    #[serde(skip)]
    _sealed: (),
}

impl QueueRuntimeConfig {
    pub fn new(default_limit: usize, retention_secs: u64, cleanup_interval_secs: u64) -> Self {
        Self {
            default_limit,
            retention_secs,
            cleanup_interval_secs,
            _sealed: (),
        }
    }
}

impl Default for QueueRuntimeConfig {
    fn default() -> Self {
        Self {
            default_limit: queue::DEFAULT_LIMIT,
            retention_secs: queue::RETENTION_SECS,
            cleanup_interval_secs: queue::CLEANUP_INTERVAL_SECS,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SessionRuntimeConfig {
    pub ttl_secs: u64,
    pub cleanup_interval_secs: u64,
    #[serde(skip)]
    _sealed: (),
}

impl SessionRuntimeConfig {
    pub fn new(ttl_secs: u64, cleanup_interval_secs: u64) -> Self {
        Self {
            ttl_secs,
            cleanup_interval_secs,
            _sealed: (),
        }
    }
}

impl Default for SessionRuntimeConfig {
    fn default() -> Self {
        Self {
            ttl_secs: session::TTL_SECS,
            cleanup_interval_secs: session::CLEANUP_INTERVAL_SECS,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct RouletteRuntimeConfig {
    pub timeout_secs: u64,
    #[serde(skip)]
    _sealed: (),
}

impl RouletteRuntimeConfig {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            timeout_secs,
            _sealed: (),
        }
    }
}

impl Default for RouletteRuntimeConfig {
    fn default() -> Self {
        Self {
            timeout_secs: roulette::TIMEOUT_SECS,
            _sealed: (),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SheetsRuntimeConfig {
    pub spreadsheet_id: String,
    #[serde(skip)]
    _sealed: (),
}

impl SheetsRuntimeConfig {
    pub fn new(spreadsheet_id: String) -> Self {
        Self {
            spreadsheet_id,
            _sealed: (),
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
    pub sheets: SheetsRuntimeConfig,
    #[serde(skip)]
    _sealed: (),
}

impl RuntimeConfig {
    pub fn new(
        widget_access_key: String,
        queue: QueueRuntimeConfig,
        session: SessionRuntimeConfig,
        roulette: RouletteRuntimeConfig,
        sheets: SheetsRuntimeConfig,
    ) -> Self {
        Self {
            widget_access_key,
            queue,
            session,
            roulette,
            sheets,
            _sealed: (),
        }
    }

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
#[path = "runtime.test.rs"]
mod tests;
