use std::time::Duration;

use crate::rt::{delay, spawn, JoinHandle};
use futures::{
    channel::mpsc::{channel, Receiver},
    select, FutureExt, StreamExt,
};

pub struct TokenBucketRateLimiter {
    receiver: Receiver<()>,
    handle: Option<Box<dyn JoinHandle>>,
}

impl TokenBucketRateLimiter {
    /// Creates a new [`TokenBucketRateLimiter`].
    ///
    /// `rate` specifies the average number of operations allowed per second.
    ///
    /// `burst` specifies the maximum burst number of operations allowed in a
    /// second.
    ///
    /// **Note**: `rate` *MUST* be little or equal than `burst`.
    pub fn new(rate: usize, burst: usize) -> TokenBucketRateLimiter {
        assert!(rate <= burst);
        let (mut sender, receiver) = channel(burst);

        let handle = spawn(async move {
            loop {
                for _ in 0..rate {
                    let _ = sender.try_send(());
                }
                delay(Duration::from_secs(1)).await;
            }
        });

        TokenBucketRateLimiter {
            receiver,
            handle: Some(Box::new(handle)),
        }
    }

    /// Acquire a token. When the token is successfully acquired, it means that
    /// you can safely perform frequency-controlled operations.
    ///
    /// If the `timeout` is not `None` and the method fails to obtain a token
    /// after exceeding the `timeout`, false will be returned.
    ///
    /// In all other cases, true will be returned.
    pub async fn acquire(&mut self, timeout: Option<Duration>) -> bool {
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

    fn close(&mut self) {
        if let Some(mut handle) = self.handle.take() {
            handle.cancel();
        }
    }
}

impl Drop for TokenBucketRateLimiter {
    fn drop(&mut self) {
        self.close();
    }
}
