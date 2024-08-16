use std::time::Duration;

use rate_limiter::{rate_limit_with_burst, RateLimiter};

#[tokio::main]
async fn main() {
    let mut rl = rate_limit_with_burst(1, 2);
    rl.start();

    rl.acquire(None).await;
    println!("Do something that requires frequency control ...");

    // Wait up to 10 seconds
    if rl.acquire(Duration::from_secs(10)).await {
        println!("Do something that requires frequency control ...");
    }

    rl.stop();
}
