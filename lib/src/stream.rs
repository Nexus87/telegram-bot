use std::cmp::max;
use std::collections::VecDeque;
use std::pin::Pin;
use std::time::Duration;

use futures::task::{Context, Poll};
use futures::{Future, FutureExt, Stream};
use telegram_bot_raw::{GetUpdates, Integer, Update};

use crate::api::Api;
use crate::errors::Error;

const TELEGRAM_LONG_POLL_TIMEOUT_SECONDS: u64 = 5;
const TELEGRAM_LONG_POLL_ERROR_DELAY_MILLISECONDS: u64 = 500;

/// This type represents stream of Telegram API updates and uses
/// long polling method under the hood.
#[must_use = "streams do nothing unless polled"]
pub struct UpdatesStream {
    api: Api,
    last_update: Integer,
    buffer: VecDeque<Update>,
    current_request: Option<Pin<Box<dyn Future<Output = Result<Option<Vec<Update>>, Error>>>>>,
    timeout: Duration,
    error_delay: Duration,
}

impl Stream for UpdatesStream {
    type Item = Result<Update, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut_ref = self.get_mut();
        if let Some(value) = mut_ref.buffer.pop_front() {
            return Poll::Ready(Some(Ok(value)));
        }

        let result = match mut_ref.current_request {
            None => Ok(false),
            Some(ref mut current_request) => {
                let polled_update = current_request.as_mut().poll(cx);
                match polled_update {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(None)) => Ok(false),
                    Poll::Ready(Ok(Some(updates))) => {
                        for update in updates {
                            mut_ref.last_update = max(update.id, mut_ref.last_update);
                            mut_ref.buffer.push_back(update)
                        }
                        Ok(true)
                    }
                    Poll::Ready(Err(err)) => Err(err),
                }
            }
        };
        match result {
            Err(err) => {
                let timeout_future = tokio::time::delay_for(mut_ref.error_delay)
                    .map(|_| Ok(None))
                    .boxed();
                mut_ref.current_request = Some(timeout_future);
                return Poll::Ready(Some(Err(err)));
            }
            Ok(false) => {
                let api = mut_ref.api.clone();
                let mut get_updates = GetUpdates::new();
                get_updates
                    .offset(mut_ref.last_update + 1)
                    .timeout(mut_ref.timeout.as_secs() as Integer);
                let timeout = mut_ref.timeout + Duration::from_secs(1);

                let request = api.send_timeout(
                    get_updates,
                    timeout,
                );

                mut_ref.current_request = Some(Box::pin(request));
                Pin::new(mut_ref).poll_next(cx)
            }
            Ok(true) => {
                mut_ref.current_request = None;
                Pin::new(mut_ref).poll_next(cx)
            }
        }
    }
}

pub trait NewUpdatesStream {
    fn new(api: Api) -> Self;
}

impl NewUpdatesStream for UpdatesStream {
    fn new(api: Api) -> Self {
        UpdatesStream {
            api,
            last_update: 0,
            buffer: VecDeque::new(),
            current_request: None,
            timeout: Duration::from_secs(TELEGRAM_LONG_POLL_TIMEOUT_SECONDS),
            error_delay: Duration::from_millis(TELEGRAM_LONG_POLL_ERROR_DELAY_MILLISECONDS),
        }
    }
}

impl UpdatesStream {
    /// Set timeout for long polling requests, this corresponds with `timeout` field
    /// in [getUpdates](https://core.telegram.org/bots/api#getupdates) method,
    /// also this stream sets an additional request timeout for `timeout + 1 second`
    /// in case of invalid Telegram API server behaviour.
    ///
    /// Default timeout is 5 seconds.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Set a delay between erroneous request and next request.
    /// This delay prevents busy looping in some cases.
    ///
    /// Default delay is 500 ms.
    pub fn error_delay(&mut self, delay: Duration) -> &mut Self {
        self.error_delay = delay;
        self
    }
}
