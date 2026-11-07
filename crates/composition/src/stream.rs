use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::nonpoison::RwLock;

use model::platform::PlatformId;
use utoipa::ToSchema;

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct StreamStatus {
    manual: AtomicBool,
    platforms: RwLock<HashMap<PlatformId, bool>>,
}

impl StreamStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_online(&self, online: bool) {
        self.manual.store(online, Ordering::Release);
    }

    pub fn set_platform(&self, platform: PlatformId, online: bool) {
        self.platforms.write().insert(platform, online);
    }

    pub fn is_online(&self) -> bool {
        self.manual.load(Ordering::Acquire) || self.platforms.read().values().any(|online| *online)
    }
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct StreamStatusResponse {
    pub online: bool,
    #[serde(skip)]
    _sealed: (),
}

impl StreamStatusResponse {
    pub fn new(online: bool) -> Self {
        Self {
            online,
            _sealed: (),
        }
    }
}

#[cfg(test)]
#[path = "stream.test.rs"]
mod tests;
