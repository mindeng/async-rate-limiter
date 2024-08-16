//! async-rate-limiter implements a [token bucket
//! algorithm](https://en.wikipedia.org/wiki/Token_bucket) that can be used to
//! limit API access frequency.
//!
//! ## Example
//!
//! Update your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! # Change features to ["rt-async-std"] if you are using async-std runtime.
//! async-rate-limiter = { version = "1.39.2", features = ["rt-tokio"] }
//! ```
//!
//! Thanks to Rust’s async functionality, this crate is very simple to use.
//! Just put your function call after [`RateLimiter::acquire()`].`await`, then
//! the function will be called with the specified rate limit.
//!
//! Here is a simple example:
//!
//! ```rust
//! use async_rate_limiter::RateLimiter;
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
//!     // acquire with a timeout
//!     let ok = rl.acquire(Some(Duration::from_secs(10))).await;
//!     if ok {
//!         println!("Do something that you want to limit the rate ...");
//!     }
//! }
//!
//! ```
//!
//! async-rate-limiter can support different async runtimes, tokio & async-std
//! are supported currently. You can use features to switch async runtimes.

mod token_bucket;
pub use token_bucket::TokenBucketRateLimiter as RateLimiter;

mod rt;

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[tokio::test]
    #[cfg(any(feature = "rt-tokio", feature = "rt-async-std"))]
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
