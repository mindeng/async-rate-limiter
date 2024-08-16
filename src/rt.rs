#[cfg(feature = "rt-tokio")]
mod rt_tokio;
#[cfg(feature = "rt-tokio")]
pub use rt_tokio::{delay, spawn};

#[cfg(feature = "rt-async-std")]
mod rt_async_std;
#[cfg(feature = "rt-async-std")]
pub use rt_async_std::{delay, spawn};

pub trait JoinHandle: Send {
    fn cancel(&mut self);
}
