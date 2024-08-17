use std::{
    sync::{
        atomic::{
            AtomicUsize,
            Ordering::{Relaxed, SeqCst},
        },
        Arc, Mutex,
    },
    task::Poll,
    time::Duration,
};

use crate::rt::{delay, interval, spawn, JoinHandle};
use futures::{future::select, pin_mut, task::AtomicWaker, Future, StreamExt};

const NANOS_PER_SEC: u64 = 1_000_000_000;

pub struct TokenBucketRateLimiter {
    inner: Arc<TokenBucketInner>,
    handle: Arc<Mutex<Box<dyn JoinHandle>>>,
    counter: Arc<AtomicUsize>,
}

struct TokenBucketInner {
    waker: AtomicWaker,
    tokens: AtomicUsize,
    burst: AtomicUsize,
}

impl Clone for TokenBucketRateLimiter {
    fn clone(&self) -> Self {
        let prev = self.counter.fetch_add(1, SeqCst);
        if prev == usize::MAX {
            panic!("cannot clone `TokenBucketRateLimiter` -- too many outstanding instances");
        }

        TokenBucketRateLimiter {
            inner: self.inner.clone(),
            handle: self.handle.clone(),
            counter: self.counter.clone(),
        }
    }
}

impl TokenBucketRateLimiter {
    /// Creates a new [`TokenBucketRateLimiter`].
    ///
    /// `rate` specifies the average number of operations allowed per second.
    ///
    /// **Note**: `rate` *MUST* be greater than zero.
    pub fn new(rate: usize) -> TokenBucketRateLimiter {
        assert!(rate > 0);
        let inner = TokenBucketInner {
            tokens: AtomicUsize::new(0),
            waker: AtomicWaker::new(),
            burst: AtomicUsize::new(rate),
        };
        let inner = Arc::new(inner);
        let handle = Self::start_fill_tokens(inner.clone(), rate);

        TokenBucketRateLimiter {
            inner,
            handle: Arc::new(Mutex::new(Box::new(handle))),
            counter: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// `burst` specifies the maximum burst number of operations allowed in a
    /// second.
    ///
    /// The default value of `burst` is same as `rate`.
    ///
    /// **Note**: `burst` *MUST* be greater than zero.
    pub fn burst(&mut self, burst: usize) -> &mut TokenBucketRateLimiter {
        assert!(burst > 0);
        self.inner.burst.store(burst, Relaxed);
        self
    }

    fn start_fill_tokens(inner: Arc<TokenBucketInner>, rate: usize) -> impl JoinHandle {
        spawn(async move {
            // TODO: tick once per second
            let mut stream = interval(Duration::from_nanos(NANOS_PER_SEC / (rate as u64)));

            while (stream.next().await).is_some() {
                println!("tick ... {:p}", inner);
                inner.inc_num_tokens();
                inner.waker.wake();
            }
        })
    }

    /// Acquire a token. When the token is successfully acquired, it means that
    /// you can safely perform frequency-controlled operations.
    ///
    /// If the `timeout` is not `None` and the method fails to obtain a token
    /// after exceeding the `timeout`, false will be returned.
    ///
    /// In all other cases, true will be returned.
    pub async fn acquire<T: Into<Option<Duration>> + std::marker::Copy>(
        &mut self,
        timeout: T,
    ) -> bool {
        loop {
            let old = self.inner.dec_num_tokens();
            if old.is_some() {
                return true;
            }

            let notify = Notify {
                token_bucket: self.inner.clone(),
            };

            let timeout: Option<Duration> = timeout.into();

            if let Some(timeout) = timeout {
                const TOLERANCE: Duration = Duration::from_millis(10);
                let delay_fut = delay(timeout + TOLERANCE);
                pin_mut!(delay_fut);
                match select(notify, delay_fut).await {
                    futures::future::Either::Left(_) => continue,
                    futures::future::Either::Right(_) => return false,
                }
            } else {
                notify.await;
                continue;
            }
        }
    }

    fn close(&mut self) {
        let mut handle = self.handle.lock().unwrap();
        handle.cancel();
    }
}

impl TokenBucketInner {
    // Increment the number of tokens and ensure that it does not exceeds
    // `burst`. Returns the resulting number.
    fn inc_num_tokens(&self) -> usize {
        let mut curr = self.tokens.load(SeqCst);
        let burst = self.burst.load(Relaxed);
        loop {
            if curr >= burst {
                return curr;
            }

            let next = curr + 1;
            match self.tokens.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => {
                    return next;
                }
                Err(actual) => curr = actual,
            }
        }
    }

    // Decrement the number of tokens and ensure that it won't be less than
    // zero. Returns the resulting number if the decrement operation is done
    // successfully.
    fn dec_num_tokens(&self) -> Option<usize> {
        let mut curr = self.tokens.load(SeqCst);
        loop {
            if curr == 0 {
                return None;
            }

            let next = curr - 1;
            match self.tokens.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => {
                    return Some(next);
                }
                Err(actual) => curr = actual,
            }
        }
    }
}

impl Drop for TokenBucketRateLimiter {
    fn drop(&mut self) {
        let prev = self.counter.fetch_sub(1, SeqCst);
        if prev == 1 {
            self.close();
            println!("dropped ... {:p}", self);
        }
    }
}

/// Notify is a future that will be completed when there is an available token
/// in the token bucket.
pub struct Notify {
    token_bucket: Arc<TokenBucketInner>,
}

impl Future for Notify {
    type Output = usize;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let num = self.token_bucket.tokens.load(SeqCst);
        if num >= 1 {
            Poll::Ready(num)
        } else {
            self.token_bucket.waker.register(cx.waker());
            Poll::Pending
        }
    }
}
