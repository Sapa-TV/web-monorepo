use std::sync::Arc;
use std::sync::nonpoison::RwLock;

use model::config::RuntimeConfig;

#[derive(Clone)]
#[non_exhaustive]
pub struct SharedSettings(Arc<RwLock<RuntimeConfig>>);

impl SharedSettings {
    pub fn new(runtime: RuntimeConfig) -> Self {
        Self(Arc::new(RwLock::new(runtime)))
    }

    pub fn read(&self) -> RuntimeConfig {
        self.0.read().clone()
    }

    pub fn set(&self, next: RuntimeConfig) {
        *self.0.write() = next;
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn test_new(runtime: RuntimeConfig) -> Self {
        Self::new(runtime)
    }
}
