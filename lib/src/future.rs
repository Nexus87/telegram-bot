use futures::{Future, Poll};

use errors::Error;

/// Represent a future that resolves into Telegram API response.
#[must_use = "futures do nothing unless polled"]
pub struct TelegramFuture<T> {
    inner: Box<Future<Item=T, Error=Error> + Send>
}

pub trait NewTelegramFuture<T> {
    fn new(inner: Box<Future<Item=T, Error=Error> + Send>) -> Self;
}

impl<T> NewTelegramFuture<T> for TelegramFuture<T> {
    fn new(inner: Box<Future<Item=T, Error=Error> + Send>) -> Self {
        Self {
            inner
        }
    }
}

// unsafe impl<T> Send for TelegramFuture<T> where T: Send {}

impl<T> Future for TelegramFuture<T> {
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}
