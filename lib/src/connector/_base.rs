use std::fmt::Debug;
use futures::Future;
use crate::errors::Error;
use std::pin::Pin;
use telegram_bot_raw::{HttpRequest, HttpResponse};

/// Connector provides basic IO with Telegram Bot API server.
pub trait Connector: Debug + Send + Sync {
    fn request(&self, token: &str, req: HttpRequest) -> Pin<Box<dyn Future<Output=Result<HttpResponse,Error>> + Send>>;
}
