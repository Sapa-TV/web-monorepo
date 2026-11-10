use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::nonpoison::RwLock;

use model::platform::PlatformId;
use tokio::sync::broadcast;
use utoipa::ToSchema;

#[derive(Debug)]
#[non_exhaustive]
pub struct StreamStatus {
    manual: AtomicBool,
    rtmp: AtomicBool,
    platforms: RwLock<HashMap<PlatformId, bool>>,
    changes: broadcast::Sender<bool>,
    ready: AtomicBool,
}

impl StreamStatus {
    pub fn new() -> Self {
        let (changes, _) = broadcast::channel(16);
        Self {
            manual: AtomicBool::new(false),
            rtmp: AtomicBool::new(false),
            platforms: RwLock::default(),
            changes,
            ready: AtomicBool::new(false),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<bool> {
        self.changes.subscribe()
    }

    pub fn mark_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }

    pub fn set_online(&self, online: bool) {
        let before = self.is_online();
        self.manual.store(online, Ordering::Release);
        self.notify(before);
    }

    pub fn set_rtmp_online(&self, online: bool) {
        let before = self.is_online();
        self.rtmp.store(online, Ordering::Release);
        self.notify(before);
    }

    pub fn set_platform(&self, platform: PlatformId, online: bool) {
        let before = self.is_online();
        self.platforms.write().insert(platform, online);
        self.notify(before);
    }

    fn notify(&self, before: bool) {
        let after = self.is_online();
        if before != after && self.ready.load(Ordering::Acquire) {
            self.changes.send(after).ok();
        }
    }

    pub fn is_online(&self) -> bool {
        self.manual.load(Ordering::Acquire)
            || self.rtmp.load(Ordering::Acquire)
            || self.platforms.read().values().any(|online| *online)
    }
}

impl Default for StreamStatus {
    fn default() -> Self {
        Self::new()
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
