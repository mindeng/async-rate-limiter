use std::time::Duration;

use crate::{
    util::{delay, spawn, JoinHandle},
    RateLimiter,
};
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    select, FutureExt, StreamExt,
};

pub struct TokenBucketRL {
    rate: usize,
    sender: Sender<()>,
    receiver: Receiver<()>,
    handle: Option<Box<dyn JoinHandle>>,
}

impl TokenBucketRL {
    /// Creates a new [`TokenBucketRL`].
    pub fn new(rate: usize, burst: usize) -> TokenBucketRL {
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
