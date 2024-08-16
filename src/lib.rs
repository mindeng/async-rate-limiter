use std::time::Duration;

mod util;

mod token_bucket;
use token_bucket::TokenBucketRL;

pub trait RateLimiter {
    fn acquire(
        &mut self,
        timeout: Option<Duration>,
    ) -> impl std::future::Future<Output = bool> + Send;
    fn start(&mut self);
    fn stop(&mut self);
}

pub fn rate_limit_with_burst(rate: usize, burst: usize) -> impl RateLimiter {
    TokenBucketRL::new(rate, burst)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[tokio::test]
    async fn test_rate_limit_with_burst() {
        let mut rl = rate_limit_with_burst(3, 5);
        rl.start();

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

        rl.stop();

        let res = rl.acquire(Some(Duration::from_secs(1))).await;
        assert!(!res);
    }
}
