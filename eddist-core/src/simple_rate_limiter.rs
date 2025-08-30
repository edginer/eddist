use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct RateLimitElem {
    count: usize,
    last_refresh: Instant,
}

#[derive(Debug)]
pub struct RateLimiter {
    limits: HashMap<String, RateLimitElem>,
    max_count: usize,
    period: Duration,
}

impl RateLimiter {
    pub fn new(max_count: usize, period: Duration) -> Self {
        Self {
            limits: HashMap::new(),
            max_count,
            period,
        }
    }

    pub fn check_and_add(&mut self, key: &str) -> bool {
        let entry = self.limits.entry(key.to_string()).or_insert(RateLimitElem {
            count: 0,
            last_refresh: Instant::now(),
        });

        if entry.last_refresh.elapsed() >= self.period {
            entry.count = 0;
            entry.last_refresh = Instant::now();
        }

        if entry.count < self.max_count {
            entry.count += 1;
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_rate_limiting() {
        let mut limiter = RateLimiter::new(2, Duration::from_secs(60));

        assert!(limiter.check_and_add("user1"));
        assert!(limiter.check_and_add("user1"));
        assert!(!limiter.check_and_add("user1"));
    }

    #[test]
    fn test_independent_keys() {
        let mut limiter = RateLimiter::new(1, Duration::from_secs(60));

        assert!(limiter.check_and_add("user1"));
        assert!(!limiter.check_and_add("user1"));
        assert!(limiter.check_and_add("user2"));
    }

    #[test]
    fn test_period_reset() {
        let mut limiter = RateLimiter::new(1, Duration::from_millis(50));

        assert!(limiter.check_and_add("user1"));
        assert!(!limiter.check_and_add("user1"));

        thread::sleep(Duration::from_millis(60));
        assert!(limiter.check_and_add("user1"));
    }
}
