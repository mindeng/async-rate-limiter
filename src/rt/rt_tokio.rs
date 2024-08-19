use futures::{Future, Stream, StreamExt};
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;

use super::JoinHandle;

pub struct JoinHandleTokio<T> {
    inner: Option<tokio::task::JoinHandle<T>>,
}

unsafe impl<T: Send> Send for JoinHandleTokio<T> {}
unsafe impl<T: Send> Sync for JoinHandleTokio<T> {}

impl<T: Send> JoinHandle for JoinHandleTokio<T> {
    fn cancel(&mut self) {
        if let Some(jh) = self.inner.take() {
            jh.abort();
        }
    }
}

pub async fn delay(duration: Duration) {
    tokio::time::sleep(duration).await
}

pub fn interval(duration: Duration) -> impl Stream<Item = ()> {
    let mut interval = tokio::time::interval(duration);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    IntervalStream::new(interval).map(|_| ())
}

pub fn spawn<F>(future: F) -> impl JoinHandle
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let handle = tokio::spawn(future);

    JoinHandleTokio {
        inner: Some(handle),
    }
}
