use std::collections::BTreeMap;
use std::sync::nonpoison::Mutex;
use std::time::{Duration, Instant};

use rand::random;

#[derive(Default)]
pub struct CsrfStore {
    pending: Mutex<BTreeMap<String, Instant>>,
}

impl CsrfStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issue(&self, ttl: Duration) -> String {
        let state = format!("{:032x}", random::<u128>());
        self.insert(state.clone(), Instant::now() + ttl);
        state
    }

    pub fn consume(&self, state: &str) -> bool {
        self.prune();
        self.pending.lock().remove(state).is_some()
    }

    pub fn insert(&self, state: String, expires_at: Instant) {
        self.pending.lock().insert(state, expires_at);
    }

    pub fn prune(&self) {
        let now = Instant::now();
        self.pending
            .lock()
            .retain(|_, expires_at| *expires_at > now);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.pending.lock().len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.pending.lock().is_empty()
    }

    #[cfg(test)]
    pub fn first_ticket(&self) -> Option<String> {
        self.pending.lock().keys().next().cloned()
    }
}

#[cfg(test)]
#[path = "csrf.test.rs"]
mod tests;
