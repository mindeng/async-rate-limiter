use std::{
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

use futures::{future::BoxFuture, Future};

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

    /// `burst` specifies the maximum burst number of operations allowed.
    ///
    /// The default value of `burst` is same as `rate`.
    ///
    /// **Note**: `burst` *MUST* be greater than zero.
    pub fn burst(&self, burst: usize) -> &TokenBucketRateLimiter {
        assert!(burst > 0);
        self.burst.store(burst, Relaxed);
        self
    }

    /// Try to acquire a token. Return `Ok` if the token is successfully
    /// acquired, it means that you can safely perform frequency-controlled
    /// operations. Otherwise `Err(duration)` is returned, `duration` is the
    /// minimum time to wait.
    pub fn try_acquire(&self) -> Result<(), Duration> {
        let mut inner = self.inner.lock().unwrap();
        match self.try_acquire_inner(&mut inner) {
            Ok(_) => Ok(()),
            Err(next) => Err(next.saturating_duration_since(Instant::now())),
        }
    }

    /// Acquire a token. When the token is successfully acquired, it means that
    /// you can safely perform frequency-controlled operations.
    pub async fn acquire(&self) {
        let need_to_wait = {
            let mut inner = self.inner.lock().unwrap();
            let Err(next) = self.try_acquire_inner(&mut inner) else {
                return;
            };

            inner.last = next;
            self.inc_num_tokens(&mut inner);
            self.dec_num_tokens(&mut inner);

            next.saturating_duration_since(Instant::now())
        };

        if !need_to_wait.is_zero() {
            Token::new(need_to_wait, self).await
        }
    }

    /// Acquire a token. When the token is successfully acquired, it means that
    /// you can safely perform frequency-controlled operations.
    ///
    /// If the method fails to obtain a token after exceeding the `timeout`,
    /// false will be returned, otherwise true will be returned.
    pub async fn acquire_with_timeout(&self, timeout: Duration) -> bool {
        let need_to_wait = {
            let mut inner = self.inner.lock().unwrap();
            let Err(next) = self.try_acquire_inner(&mut inner) else {
                return true;
            };

            inner.last = next;
            self.inc_num_tokens(&mut inner);
            self.dec_num_tokens(&mut inner);

            let need_to_wait = next.saturating_duration_since(Instant::now());
            if need_to_wait > timeout {
                // failed with timeout
                return false;
            }

            inner.last = next;
            self.inc_num_tokens(&mut inner);
            self.dec_num_tokens(&mut inner);

            if need_to_wait.is_zero() {
                return true;
            }

            need_to_wait
        };

        Token::new(need_to_wait, self).await;
        true
    }

    fn try_acquire_inner(&self, inner: &mut MutexGuard<TokenBucketInner>) -> Result<(), Instant> {
        if let Some(remain) = inner.tokens.checked_sub(1) {
            inner.tokens = remain;
            return Ok(());
        }

        match self.tokens_since_last(inner) {
            Ok((tokens, duration)) => {
                self.set_num_tokens(inner, tokens);
                inner.last += duration;
                // consume 1 token
                self.dec_num_tokens(inner);
                Ok(())
            }
            Err(duration) => Err(inner.last + duration),
        }
    }

    // Get tokens generated since `last` time. Return:
    //
    // - Ok((tokens, duration)) if tokens has been generated, duration is the time it
    //   takes to generate the token.
    //
    // - Err(duration) if tokens hasn't been generate yet, need to wait until
    //   next cycle, duration is the period of the cycle.
    fn tokens_since_last(
        &self,
        inner: &MutexGuard<TokenBucketInner>,
    ) -> Result<(usize, Duration), Duration> {
        let now = Instant::now();
        let since_last = now
            .checked_duration_since(inner.last)
            .unwrap_or(Duration::ZERO);
        let since_nanos = since_last.as_nanos();
        if since_nanos >= self.period_ns as u128 {
            let tokens = since_nanos / (self.period_ns as u128);
            assert!(tokens >= 1);
            Ok((
                tokens as usize,
                Duration::from_nanos(tokens as u64 * self.period_ns),
            ))
        } else {
            Err(Duration::from_nanos(self.period_ns))
        }
    }

    fn set_num_tokens(&self, inner: &mut MutexGuard<TokenBucketInner>, num: usize) {
        inner.tokens = std::cmp::min(self.burst.load(Relaxed), num);
    }

    fn dec_num_tokens(&self, inner: &mut MutexGuard<TokenBucketInner>) -> Option<usize> {
        if let Some(num) = inner.tokens.checked_sub(1) {
            inner.tokens = num;
            Some(num)
        } else {
            None
        }
    }

    fn inc_num_tokens(&self, inner: &mut MutexGuard<TokenBucketInner>) -> usize {
        if let Some(num) = inner.tokens.checked_add(1) {
            self.set_num_tokens(inner, num);
        }
        inner.tokens
    }
}

struct Token<'a> {
    fut: BoxFuture<'a, ()>,
    token_bucket: &'a TokenBucketRateLimiter,
    consumed: bool,
}

impl<'a> Token<'a> {
    fn new(duration: Duration, token_bucket: &'a TokenBucketRateLimiter) -> Token<'a> {
        let fut = delay(duration);
        let fut = Box::pin(fut);
        Self {
            fut,
            token_bucket,
            consumed: false,
        }
    }
}

impl Future for Token<'_> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match this.fut.as_mut().poll(cx) {
            std::task::Poll::Ready(_) => {
                this.consumed = true;
                Poll::Ready(())
            }
            std::task::Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for Token<'_> {
    fn drop(&mut self) {
        if self.consumed {
            return;
        }

        // Return unused token
        let mut inner = self.token_bucket.inner.lock().unwrap();
        let num = inner.tokens + 1;
        self.token_bucket.set_num_tokens(&mut inner, num);
    }
}
