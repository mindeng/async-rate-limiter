use futures::Future;
use std::time::Duration;

pub trait JoinHandle: Send {
    fn cancel(&mut self);
}

pub struct JoinHandleTokio<T> {
    inner: tokio::task::JoinHandle<T>,
}

unsafe impl<T: Send> Send for JoinHandleTokio<T> {}

impl<T: Send> JoinHandle for JoinHandleTokio<T> {
    fn cancel(&mut self) {
        self.inner.abort();
    }
}

pub async fn delay(duration: Duration) {
    tokio::time::sleep(duration).await
}

pub fn spawn<F>(future: F) -> impl JoinHandle
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let handle = tokio::spawn(future);
    JoinHandleTokio { inner: handle }
}
