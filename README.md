# async-rate-limiter

[![crates.io](https://img.shields.io/crates/v/async-rate-limiter.svg)](https://crates.io/crates/async-rate-limiter)
[![Documentation](https://docs.rs/async-rate-limiter/badge.svg)](https://docs.rs/async-rate-limiter)
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/mindeng/async-rate-limiter/actions/workflows/rust.yml/badge.svg)](https://github.com/mindeng/async-rate-limiter/actions)

async-rate-limiter implements a [token bucket
algorithm](https://en.wikipedia.org/wiki/Token_bucket) that can be used to
limit API access frequency.

## Features

- Simple to use
- Support concurrent access
- Low overhead, the number of tokens is calculated over time

Thanks to Rust’s `async` / `await`, this crate is very simple to use. Just put
your function call after
[`RateLimiter::acquire()`](https://docs.rs/async-rate-limiter/latest/async_rate_limiter/struct.RateLimiter.html#method.acquire).`await`,
then the function will be called with the specified rate limit.

[`RateLimiter`] also implements `Clone` trait, so you can use it in
multiple tasks environment easily.

## Example

Update your `Cargo.toml`:

```toml
[dependencies]
# Change features to ["rt-async-std"] if you are using async-std runtime.
async-rate-limiter = { version = "1", features = ["rt-tokio"] }
```

Here is a simple example:

```rust
use async_rate_limiter::RateLimiter;
use std::time::Duration;
use tokio::spawn;

#[tokio::main]
async fn main() {
    let rl = RateLimiter::new(3);
    // You can change `burst` at anytime
    rl.burst(5);
    
    rl.acquire().await;
    println!("Do something that you want to limit the rate ...");

    let res = rl.try_acquire();
    if res.is_ok() {
        println!("Do something that you want to limit the rate ...");
    }

    // acquire with a timeout
    let ok = rl.acquire_with_timeout(Duration::from_secs(10)).await;
    if ok {
        println!("Do something that you want to limit the rate ...");
    }

    // Concurrent use
    let rl = rl.clone();
    spawn(async move {
        let res = rl.acquire_with_timeout(Duration::from_millis(340)).await;
        assert!(res);
    });
}

```

async-rate-limiter can support different async runtimes, tokio & async-std
are supported currently. You can use features to switch async runtimes.
