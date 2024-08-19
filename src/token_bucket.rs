use std::{
    pin::Pin,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering::{Relaxed, SeqCst},
        },
        Arc, Mutex, MutexGuard,
    },
    task::Poll,
    time::{Duration, Instant},
};

use futures::{Future, FutureExt};

use crate::rt::delay;

const NANOS_PER_SEC: u64 = 1_000_000_000;

pub struct TokenBucketRateLimiter {
    inner: Arc<Mutex<TokenBucketInner>>,
    counter: Arc<AtomicUsize>,
    burst: Arc<AtomicUsize>,
    period_ns: u64,
}

struct TokenBucketInner {
    tokens: usize,
    last: Instant,
}

impl Clone for TokenBucketRateLimiter {
    fn clone(&self) -> Self {
        let prev = self.counter.fetch_add(1, SeqCst);
        if prev == usize::MAX {
            panic!("cannot clone `TokenBucketRateLimiter` -- too many outstanding instances");
        }

        TokenBucketRateLimiter {
            inner: self.inner.clone(),
            counter: self.counter.clone(),
            burst: self.burst.clone(),
            period_ns: self.period_ns,
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

        let period_ns = NANOS_PER_SEC.checked_div(rate as u64).unwrap();

        let inner = TokenBucketInner {
            tokens: 1,
            last: Instant::now(),
        };
        let inner = Arc::new(Mutex::new(inner));

        TokenBucketRateLimiter {
            inner,
            counter: Arc::new(AtomicUsize::new(1)),
            burst: Arc::new(AtomicUsize::new(rate)),
            period_ns,
        }
    }

    /// `burst` specifies the maximum burst number of operations allowed in a
    /// second.
    ///
    /// The default value of `burst` is same as `rate`.
    ///
    /// **Note**: `burst` *MUST* be greater than zero.
    pub fn burst(&self, burst: usize) -> &TokenBucketRateLimiter {
        assert!(burst > 0);
        self.burst.store(burst, Relaxed);
        self
    }

    /// Acquire a token. When the token is successfully acquired, it means that
    /// you can safely perform frequency-controlled operations.
    pub async fn acquire(&self) {
        let ready = {
            let mut inner = self.inner.lock().unwrap();
            if let Some(remain) = inner.tokens.checked_sub(1) {
                inner.tokens = remain;
                return;
            }

            // Update tokens & next
            if let Err(Some(ready)) = self.update_tokens(inner, None) {
                ready
            } else {
                return;
            }
        };

        // delay(ready).await
        Delay::new(ready, self).await;
    }

    /// Acquire a token. When the token is successfully acquired, it means that
    /// you can safely perform frequency-controlled operations.
    ///
    /// If the method fails to obtain a token after exceeding the `timeout`,
    /// false will be returned, otherwise true will be returned.
    pub async fn acquire_with_timeout(&self, timeout: Duration) -> bool {
        let ready = {
            let mut inner = self.inner.lock().unwrap();
            if let Some(remain) = inner.tokens.checked_sub(1) {
                inner.tokens = remain;
                return true;
            }

            // Update tokens & next
            match self.update_tokens(inner, Some(timeout)) {
                Err(Some(ready)) => ready, // to wait
                Err(None) => return false, // timeout
                Ok(_) => return true,      // got token
            }
        };

        if ready > timeout {
            return false;
        }

        // delay(ready).await;
        Delay::new(ready, self).await;
        true
    }

    // Update tokens & next. Return:
    // - Ok if token is available, otherwise return
    // - Err(duration) if timeout is None or timeout is not exceeded,
    //   the duration is the time need to wait
    // - Err(None) if timeout is exceeded
    fn update_tokens(
        &self,
        mut inner: MutexGuard<TokenBucketInner>,
        timeout: Option<Duration>,
    ) -> Result<(), Option<Duration>> {
        let now = Instant::now();
        let since_last = now
            .checked_duration_since(inner.last)
            .unwrap_or(Duration::ZERO);
        let since_nanos = since_last.as_nanos();
        if since_nanos >= self.period_ns as u128 {
            let extra_tokens = since_nanos / (self.period_ns as u128);
            assert!(extra_tokens >= 1);
            inner.tokens = std::cmp::min(self.burst.load(Relaxed), extra_tokens as usize)
                .checked_sub(1)
                .unwrap();
            inner.last += Duration::from_nanos((extra_tokens * self.period_ns as u128) as u64);
            Ok(())
        } else {
            inner.last += Duration::from_nanos(self.period_ns);
            inner.tokens = 1;
            let need_to_wait = inner.last - now;
            if let Some(timeout) = timeout {
                if timeout >= need_to_wait {
                    inner.tokens = 0;
                    Err(Some(need_to_wait))
                } else {
                    Err(None)
                }
            } else {
                inner.tokens = 0;
                Err(Some(need_to_wait))
            }
        }
    }
}

struct Delay<'a> {
    fut: Pin<Box<dyn Future<Output = ()>>>,
    token_bucket: &'a TokenBucketRateLimiter,
    consumed: bool,
}

unsafe impl Send for Delay<'_> {}

impl<'a> Delay<'a> {
    fn new(duration: Duration, token_bucket: &'a TokenBucketRateLimiter) -> Delay<'a> {
        let fut = delay(duration);
        let fut = Box::pin(fut);
        Self {
            fut,
            token_bucket,
            consumed: false,
        }
    }
}

impl Future for Delay<'_> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match this.fut.poll_unpin(cx) {
            std::task::Poll::Ready(_) => {
                this.consumed = true;
                Poll::Ready(())
            }
            std::task::Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for Delay<'_> {
    fn drop(&mut self) {
        if self.consumed {
            return;
        }

        // Return unused token
        let mut inner = self.token_bucket.inner.lock().unwrap();
        inner.tokens += 1;
    }
}
