use std::collections::{HashMap, VecDeque};
use std::sync::nonpoison::Mutex;
use std::time::{Duration, Instant};

pub struct GuestRateLimiter {
    limit: usize,
    window: Duration,
    attempts: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl GuestRateLimiter {
    pub fn new(limit: usize, window: Duration) -> Self {
        Self {
            limit,
            window,
            attempts: Mutex::new(HashMap::new()),
        }
    }

    pub fn allow(&self, key: &str) -> bool {
        self.allow_at(key, Instant::now())
    }

    fn allow_at(&self, key: &str, now: Instant) -> bool {
        let mut attempts = self.attempts.lock();
        if attempts.len() > 10_000 {
            attempts.retain(|_, queue| {
                queue
                    .back()
                    .is_some_and(|last| now.duration_since(*last) < self.window)
            });
        }
        let queue = attempts.entry(key.to_string()).or_default();
        while queue
            .front()
            .is_some_and(|at| now.duration_since(*at) >= self.window)
        {
            queue.pop_front();
        }
        if queue.len() >= self.limit {
            return false;
        }
        queue.push_back(now);
        true
    }
}

#[cfg(test)]
#[path = "rate_limit.test.rs"]
mod tests;
