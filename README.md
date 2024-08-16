# rate-limiter

[![crates.io](https://img.shields.io/crates/v/rate-limiter.svg)](https://crates.io/crates/rate-limiter)
[![Documentation](https://docs.rs/rate-limiter/badge.svg)](https://docs.rs/rate-limiter)
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/mindeng/rate-limiter/actions/workflows/rust.yml/badge.svg)](https://github.com/mindeng/rate-limiter/actions)

rate-limiter implements a [token bucket
algorithm](https://en.wikipedia.org/wiki/Token_bucket) that can be used to
limit API access frequency.

Thanks to Rust’s async capabilities, this feature is very simple to use. Just
put your operations after [`RateLimiter::acquire()`].`await`, then the
frequency control of the operations has been done.

Here is a simple example:

```rust
use rate_limiter::RateLimiter;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut rl = RateLimiter::new(3, 5);
    
    let ok = rl.acquire(None).await;
    assert!(ok);
    println!("Do something that you want to limit the rate ...");
    
    let ok = rl.acquire(Some(Duration::from_secs(10))).await;
    if ok {
        println!("Do something that you want to limit the rate ...");
    }
}
```
