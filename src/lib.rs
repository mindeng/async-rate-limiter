use std::time::Duration;

use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    select, FutureExt, StreamExt,
};

mod util;
use util::{delay, spawn, JoinHandle};

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

struct TokenBucketRL {
    rate: usize,
    sender: Sender<()>,
    receiver: Receiver<()>,
    handle: Option<Box<dyn JoinHandle>>,
}

impl TokenBucketRL {
    /// Creates a new [`TokenBucketRL`].
    fn new(rate: usize, burst: usize) -> TokenBucketRL {
        let (sender, receiver) = channel(burst);
        TokenBucketRL {
            rate,
            sender,
            receiver,
            handle: None,
        }
    }
}

impl RateLimiter for TokenBucketRL {
    async fn acquire(&mut self, timeout: Option<Duration>) -> bool {
        let mut next = self.receiver.next().fuse();
        if let Some(timeout) = timeout {
            const TOLERANCE: Duration = Duration::from_millis(10);
            select! {
                _ = next => {
                    true
                },
                _ = delay(timeout+TOLERANCE).fuse() => {
                    false
                }
            }
        } else {
            next.await;
            true
        }
    }

    fn start(&mut self) {
        let mut sender = self.sender.clone();
        let rate = self.rate;
        let handle = spawn(async move {
            loop {
                #[cfg(debug_assertions)]
                println!("Fill tokens.");

                for _ in 0..rate {
                    let _ = sender.try_send(());
                }
                delay(Duration::from_secs(1)).await;
            }
        });
        self.handle = Some(Box::new(handle));
    }

    fn stop(&mut self) {
        if let Some(mut handle) = self.handle.take() {
            handle.cancel();
        }
    }
}

impl Drop for TokenBucketRL {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[tokio::test]
    async fn acquire() {
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
