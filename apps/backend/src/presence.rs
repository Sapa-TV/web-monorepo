use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::watch;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, strum::AsRefStr,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[non_exhaustive]
pub enum WsClientRole {
    Dock,
    Widget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PresenceSnapshot {
    pub dock: usize,
    pub widget: usize,
    _sealed: (),
}

impl PresenceSnapshot {
    pub fn new(dock: usize, widget: usize) -> Self {
        Self {
            dock,
            widget,
            _sealed: (),
        }
    }
}

#[non_exhaustive]
pub struct Presence {
    dock: AtomicUsize,
    widget: AtomicUsize,
    tx: watch::Sender<PresenceSnapshot>,
}

impl Presence {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(PresenceSnapshot::default());
        Arc::new(Self {
            dock: AtomicUsize::new(0),
            widget: AtomicUsize::new(0),
            tx,
        })
    }

    pub fn add(self: &Arc<Self>, role: WsClientRole) -> PresenceGuard {
        let counter = match role {
            WsClientRole::Dock => &self.dock,
            WsClientRole::Widget => &self.widget,
        };
        counter.fetch_add(1, Ordering::AcqRel);
        self.publish();
        PresenceGuard {
            presence: Arc::clone(self),
            role,
        }
    }

    pub fn snapshot(&self) -> PresenceSnapshot {
        PresenceSnapshot::new(
            self.dock.load(Ordering::Acquire),
            self.widget.load(Ordering::Acquire),
        )
    }

    pub fn subscribe(&self) -> watch::Receiver<PresenceSnapshot> {
        self.tx.subscribe()
    }

    fn publish(&self) {
        self.tx.send_replace(self.snapshot());
    }
}

#[non_exhaustive]
pub struct PresenceGuard {
    presence: Arc<Presence>,
    role: WsClientRole,
}

impl Drop for PresenceGuard {
    fn drop(&mut self) {
        let counter = match self.role {
            WsClientRole::Dock => &self.presence.dock,
            WsClientRole::Widget => &self.presence.widget,
        };
        let prev = counter.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(prev > 0, "presence guard dropped more times than added");
        self.presence.publish();
    }
}

#[cfg(test)]
#[path = "presence.test.rs"]
mod tests;
