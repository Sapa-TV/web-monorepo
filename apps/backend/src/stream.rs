use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct StreamStatus {
    online: AtomicBool,
}

impl StreamStatus {
    pub fn new() -> Self {
        Self {
            online: AtomicBool::new(false),
        }
    }

    pub fn set_online(&self, online: bool) {
        self.online.store(online, Ordering::Release);
    }

    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Acquire)
    }
}
