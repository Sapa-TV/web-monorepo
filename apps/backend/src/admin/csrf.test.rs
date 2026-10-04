use std::time::Duration;

use super::*;

#[test]
fn issued_state_consumes_once() {
    let store = CsrfStore::new();
    let state = store.issue(Duration::from_secs(60));
    assert_eq!(store.len(), 1);
    assert!(store.consume(&state));
    assert!(!store.consume(&state));
    assert_eq!(store.len(), 0);
}

#[test]
fn issued_states_are_unique() {
    let store = CsrfStore::new();
    let first = store.issue(Duration::from_secs(60));
    let second = store.issue(Duration::from_secs(60));
    assert_ne!(first, second);
    assert_eq!(store.len(), 2);
}

#[test]
fn expired_state_is_pruned_and_rejected() {
    let store = CsrfStore::new();
    store.insert("stale".to_string(), Instant::now() - Duration::from_secs(1));
    assert!(!store.consume("stale"));
    assert_eq!(store.len(), 0);
}

#[test]
fn unknown_state_is_rejected() {
    let store = CsrfStore::new();
    assert!(!store.consume("nope"));
}
