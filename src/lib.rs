//! rate-limiter implements a token bucket algorithm that can be used to limit
//! API access frequency.
//!
//! Thanks to Rust’s async capabilities, this feature is very simple to use.
//! Just put your operations after [`RateLimiter::acquire()`].`await`, then the
//! frequency control of the operations has been done.
//!
//! Here is a simple example:
//!
//! ```rust
//! use rate_limiter::RateLimiter;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut rl = RateLimiter::new(3, 5);
//!     
//!     let ok = rl.acquire(None).await;
//!     assert!(ok);
//!     println!("Do something that you want to limit the rate ...");
//!     
//!     let ok = rl.acquire(Some(Duration::from_secs(10))).await;
//!     if ok {
//!         println!("Do something that you want to limit the rate ...");
//!     }
//! }
//!
//! ```

mod token_bucket;
pub use token_bucket::TokenBucketRateLimiter as RateLimiter;

mod util;

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[tokio::test]
    async fn test_rate_limit() {
        let mut rl = RateLimiter::new(3, 5);

        let start = Instant::now();
        let res = rl.acquire(None).await;
        assert!(res);
        let res = rl.acquire(None).await;
        assert!(res);
        let res = rl.acquire(None).await;
        assert!(res);
        assert!(start.elapsed() < Duration::from_millis(100));

        let res = rl.acquire(Some(Duration::from_secs(1))).await;
        assert!(res);
        let res = rl.acquire(None).await;
        assert!(res);
        let res = rl.acquire(None).await;
        assert!(res);
        assert!(start.elapsed() >= Duration::from_secs(1));
        assert!(start.elapsed() < Duration::from_secs(1) + Duration::from_millis(100));

        let res = rl.acquire(Some(Duration::from_millis(10))).await;
        assert!(!res);
        assert!(start.elapsed() < Duration::from_secs(1) + Duration::from_millis(100));
    }
}
