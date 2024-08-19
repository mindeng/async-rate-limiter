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
//! async-rate-limiter = { version = "1", features = ["rt-tokio"] }
//! ```
//!
//! Thanks to Rust’s async functionality, this crate is very simple to use.
//! Just put your function call after [`RateLimiter::acquire_with_timeout()`].`await`, then
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
//!     let mut rl = RateLimiter::new(3);
//!     rl.burst(5);
//!     
//!     rl.acquire().await;
//!     println!("Do something that you want to limit the rate ...");
//!
//!     // acquire with a timeout
//!     let ok = rl.acquire_with_timeout(Duration::from_secs(10)).await;
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
    use tokio::spawn;

    use super::*;

    // When using async-std runtime, the delay/interval will be longer than
    // expected, so the time check condition is deliberately relaxed in the
    // test case.

    #[tokio::test]
    #[cfg(any(feature = "rt-tokio", feature = "rt-async-std"))]
    async fn test_acquire_with_timeout() {
        let mut rl = RateLimiter::new(3);
        rl.burst(5);

        let start = Instant::now();
        rl.acquire().await;
        assert!(start.elapsed() < Duration::from_millis(10));
        rl.acquire().await;
        assert!(start.elapsed() > Duration::from_millis(330));
        assert!(start.elapsed() < Duration::from_millis(340));
        rl.acquire().await;
        assert!(start.elapsed() > Duration::from_millis(660));
        assert!(start.elapsed() < Duration::from_millis(680));

        let res = rl.acquire_with_timeout(Duration::from_millis(5000)).await;
        assert!(res);
        assert!(
            start.elapsed() >= Duration::from_secs(1),
            "got: {:?}",
            start.elapsed()
        );
        assert!(start.elapsed() < Duration::from_millis(1030));

        let res = rl.acquire_with_timeout(Duration::from_millis(10)).await;
        assert!(!res);
        assert!(start.elapsed() < Duration::from_millis(1050));
    }

    #[tokio::test]
    #[cfg(any(feature = "rt-tokio", feature = "rt-async-std"))]
    async fn test_clone() {
        let mut rl = RateLimiter::new(3);
        rl.burst(5);

        let start = Instant::now();
        rl.acquire().await;
        assert!(start.elapsed() < Duration::from_millis(10));
        rl.acquire().await;
        assert!(start.elapsed() > Duration::from_millis(330));
        assert!(start.elapsed() < Duration::from_millis(340));
        rl.acquire().await;
        assert!(start.elapsed() > Duration::from_millis(660));
        assert!(start.elapsed() < Duration::from_millis(680));

        let rl2 = rl.clone();
        let jh = spawn(async move {
            let mut rl = rl2;
            let start = Instant::now();
            let res = rl.acquire_with_timeout(Duration::from_millis(700)).await;
            assert!(res);
            assert!(
                start.elapsed() <= Duration::from_millis(700),
                "got: {:?}",
                start.elapsed()
            );
        });

        let res = rl.acquire_with_timeout(Duration::from_millis(700)).await;
        assert!(res);

        assert!(jh.await.is_ok());
    }

    #[tokio::test]
    #[cfg(any(feature = "rt-tokio", feature = "rt-async-std"))]
    async fn test_cancel_task() {
        use rt::delay;

        let mut rl = RateLimiter::new(3);
        rl.burst(5);

        let start = Instant::now();
        rl.acquire().await;
        assert!(start.elapsed() < Duration::from_millis(10));
        rl.acquire().await;
        assert!(start.elapsed() > Duration::from_millis(330));
        assert!(start.elapsed() < Duration::from_millis(340));
        rl.acquire().await;
        assert!(start.elapsed() > Duration::from_millis(660));
        assert!(start.elapsed() < Duration::from_millis(680));

        let rl2 = rl.clone();
        let jh = spawn(async move {
            let mut rl = rl2;
            let start = Instant::now();
            let res = rl.acquire_with_timeout(Duration::from_millis(700)).await;
            assert!(res);
            assert!(start.elapsed() <= Duration::from_millis(700));
        });
        delay(Duration::from_millis(100)).await;
        jh.abort();

        let start = Instant::now();
        let res = rl.acquire_with_timeout(Duration::from_millis(5000)).await;
        assert!(res);
        assert!(
            start.elapsed() <= Duration::from_millis(700),
            "got: {:?}",
            start.elapsed()
        );
    }
}
