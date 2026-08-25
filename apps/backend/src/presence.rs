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
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    #[test]
    fn add_and_drop_track_each_role_independently() {
        let presence = Presence::new();

        let dock = presence.add(WsClientRole::Dock);
        assert_eq!(presence.snapshot().dock, 1);
        assert_eq!(presence.snapshot().widget, 0);

        let widget = presence.add(WsClientRole::Widget);
        assert_eq!(presence.snapshot().widget, 1);

        drop(dock);
        drop(widget);
        assert_eq!(presence.snapshot(), PresenceSnapshot::default());
    }

    #[test]
    fn multiple_widgets_counted() {
        let presence = Presence::new();

        let first = presence.add(WsClientRole::Widget);
        let second = presence.add(WsClientRole::Widget);
        assert_eq!(presence.snapshot().widget, 2);

        drop(first);
        assert_eq!(presence.snapshot().widget, 1);

        drop(second);
        assert_eq!(presence.snapshot().widget, 0);
    }

    #[tokio::test]
    async fn subscribers_notified_on_changes_only() {
        let presence = Presence::new();
        let mut rx = presence.subscribe();

        let dock_guard = presence.add(WsClientRole::Dock);
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), PresenceSnapshot::new(1, 0));

        let widget_guard = presence.add(WsClientRole::Widget);
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), PresenceSnapshot::new(1, 1));

        drop(dock_guard);
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), PresenceSnapshot::new(0, 1));

        drop(widget_guard);
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), PresenceSnapshot::default());

        // No further changes: borrow_and_update stays current, timeout proves quiet.
        rx.borrow_and_update();
        let quiet = timeout(Duration::from_millis(50), rx.changed()).await;
        assert!(quiet.is_err(), "unexpected presence change");
    }
}
