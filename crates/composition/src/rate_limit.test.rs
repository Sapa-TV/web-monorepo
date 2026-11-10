use std::time::{Duration, Instant};

use super::GuestRateLimiter;

#[test]
fn rejects_after_limit() {
    let limiter = GuestRateLimiter::new(2, Duration::from_secs(60));
    assert!(limiter.allow("ip"));
    assert!(limiter.allow("ip"));
    assert!(!limiter.allow("ip"));
    assert!(limiter.allow("other"));
}

#[test]
fn window_expiry_allows_again() {
    let limiter = GuestRateLimiter::new(1, Duration::from_secs(60));
    let start = Instant::now();
    assert!(limiter.allow_at("ip", start));
    assert!(!limiter.allow_at("ip", start + Duration::from_secs(30)));
    assert!(limiter.allow_at("ip", start + Duration::from_secs(61)));
}
