#[cfg(feature = "rt-tokio")]
mod rt_tokio;
#[cfg(feature = "rt-tokio")]
pub use rt_tokio::{delay, interval, spawn};

#[cfg(feature = "rt-async-std")]
mod rt_async_std;
#[cfg(not(feature = "rt-tokio"))]
#[cfg(feature = "rt-async-std")]
#[allow(unused_imports)]
pub use rt_async_std::{delay, interval, spawn};

pub trait JoinHandle: Send + Sync {
    fn cancel(&mut self);
}
