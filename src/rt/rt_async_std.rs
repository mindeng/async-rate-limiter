use std::time::Duration;

// use super::JoinHandle;

// pub struct JoinHandleAsyncStd<T> {
//     inner: Option<async_std::task::JoinHandle<T>>,
// }

// unsafe impl<T: Send> Send for JoinHandleAsyncStd<T> {}
// unsafe impl<T: Send> Sync for JoinHandleAsyncStd<T> {}

// impl<T: Send + 'static> JoinHandle for JoinHandleAsyncStd<T> {
//     fn cancel(&mut self) {
//         if let Some(handle) = self.inner.take() {
//             async_std::task::spawn(async move { handle.cancel().await });
//         }
//     }
// }

#[allow(dead_code)]
pub async fn delay(duration: Duration) {
    async_std::task::sleep(duration).await
}

// pub fn interval(duration: Duration) -> impl Stream<Item = ()> {
//     async_std::stream::interval(duration)
// }

// pub fn spawn<F>(future: F) -> impl JoinHandle
// where
//     F: Future + Send + 'static,
//     F::Output: Send + 'static,
// {
//     let handle = async_std::task::spawn(future);
//     JoinHandleAsyncStd {
//         inner: Some(handle),
//     }
// }
