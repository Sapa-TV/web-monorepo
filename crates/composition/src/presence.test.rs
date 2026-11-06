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
