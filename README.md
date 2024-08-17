# async-rate-limiter

[![crates.io](https://img.shields.io/crates/v/async-rate-limiter.svg)](https://crates.io/crates/async-rate-limiter)
[![Documentation](https://docs.rs/async-rate-limiter/badge.svg)](https://docs.rs/async-rate-limiter)
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/mindeng/async-rate-limiter/actions/workflows/rust.yml/badge.svg)](https://github.com/mindeng/async-rate-limiter/actions)

async-rate-limiter implements a [token bucket
algorithm](https://en.wikipedia.org/wiki/Token_bucket) that can be used to
limit API access frequency.

## Example

Update your `Cargo.toml`:

```toml
[dependencies]
# Change features to ["rt-async-std"] if you are using async-std runtime.
async-rate-limiter = { version = "1", features = ["rt-tokio"] }
```

Thanks to Rust’s async functionality, this crate is very simple to use. Just
put your function call after
[`RateLimiter::acquire()`](https://docs.rs/async-rate-limiter/latest/async_rate_limiter/struct.RateLimiter.html#method.acquire).`await`,
then the function will be called with the specified rate limit.

Here is a simple example:

```rust
use async_rate_limiter::RateLimiter;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut rl = RateLimiter::new(3);
    rl.burst(5);
    
    rl.acquire().await;
    println!("Do something that you want to limit the rate ...");

    // acquire with a timeout
    let ok = rl.acquire_with_timeout(Duration::from_secs(10)).await;
    if ok {
        println!("Do something that you want to limit the rate ...");
    }
}

```

async-rate-limiter can support different async runtimes, tokio & async-std
are supported currently. You can use features to switch async runtimes.
