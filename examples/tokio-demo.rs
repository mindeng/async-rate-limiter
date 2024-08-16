use std::time::Duration;

use rate_limiter::RateLimiter;

#[tokio::main]
async fn main() {
    let mut rl = RateLimiter::new(3, 5);

    rl.acquire(None).await;
    println!("Do something that requires frequency control ...");

    // Wait up to 10 seconds
    if rl.acquire(Some(Duration::from_secs(10))).await {
        println!("Do something that requires frequency control ...");
    }
}
