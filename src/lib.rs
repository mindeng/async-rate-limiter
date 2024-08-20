//! async-rate-limiter implements a [token bucket
//! algorithm](https://en.wikipedia.org/wiki/Token_bucket) that can be used to
//! limit API access frequency.
//!
//! ## Features
//!
//! - Simple to use
//! - Support concurrent access
//! - Low overhead, the number of tokens is calculated over time
//!
//! Thanks to Rust’s `async` / `await`, this crate is very simple to use. Just
//! put your function call after [`RateLimiter::acquire()`].`await`, then the
//! function will be called with the specified rate limit.
//!
//! [`RateLimiter`] also implements `Clone` trait, so you can use it in
//! multiple tasks environment easily.
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
//! Here is a simple example:
//!
//! ```rust
//! use async_rate_limiter::RateLimiter;
//! use std::time::Duration;
//! use tokio::spawn;
//!
//! #[tokio::main]
//! async fn main() {
//!     let rl = RateLimiter::new(3);
//!     // You can change `burst` at anytime
//!     rl.burst(5);
//!     
//!     rl.acquire().await;
//!     println!("Do something that you want to limit the rate ...");
//!
//!     let res = rl.try_acquire();
//!     if res.is_ok() {
//!         println!("Do something that you want to limit the rate ...");
//!     }
//!
//!     // acquire with a timeout
//!     let ok = rl.acquire_with_timeout(Duration::from_secs(10)).await;
//!     if ok {
//!         println!("Do something that you want to limit the rate ...");
//!     }
//!
//!     // Concurrent use
//!     let rl = rl.clone();
//!     spawn(async move {
//!         let res = rl.acquire_with_timeout(Duration::from_millis(340)).await;
//!         assert!(res);
//!     });
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

    // When using async-std runtime, the delay/interval seems to be longer than
    // expected, so the time check condition is deliberately relaxed in the
    // test case.

    #[tokio::test]
    #[cfg(any(feature = "rt-tokio", feature = "rt-async-std"))]
    async fn test_try_acquire() {
        use rt::delay;

        let rl = RateLimiter::new(3);
        rl.burst(5);

        rl.try_acquire().unwrap();

        let duration = rl.try_acquire().unwrap_err();
        assert!(duration > Duration::from_millis(330));
        assert!(duration < Duration::from_millis(340));

        delay(duration).await;

        rl.try_acquire().unwrap();
    }

    #[tokio::test]
    #[cfg(any(feature = "rt-tokio", feature = "rt-async-std"))]
    async fn test_acquire() {
        let rl = RateLimiter::new(3);
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
        let rl = RateLimiter::new(3);
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
            let rl = rl2;
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

        let rl = RateLimiter::new(3);
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
            let rl = rl2;
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

        let start = Instant::now();
        let res = rl.acquire_with_timeout(Duration::from_millis(5000)).await;
        assert!(res);
        assert!(
            start.elapsed() <= Duration::from_millis(10),
            "got: {:?}",
            start.elapsed()
        );
    }
}
