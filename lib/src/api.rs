use crate::connector::{default_connector, Connector};
use crate::errors::Error;
use crate::stream::{NewUpdatesStream, UpdatesStream};
use futures::{Future};
use std::sync::Arc;
use std::time::Duration;
use telegram_bot_raw::{Request, ResponseType, HttpRequest};

/// Main type for sending requests to the Telegram bot API.
#[derive(Clone)]
pub struct Api(Arc<ApiInner>);

struct ApiInner {
    token: String,
    connector: Box<dyn Connector>,
}

#[derive(Debug)]
pub enum ConnectorConfig {
    Default,
    Specified(Box<dyn Connector>),
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        ConnectorConfig::Default
    }
}

impl ConnectorConfig {
    pub fn new(connector: Box<dyn Connector>) -> Self {
        ConnectorConfig::Specified(connector)
    }

    pub fn take(self) -> Result<Box<dyn Connector>, Error> {
        match self {
            ConnectorConfig::Default => default_connector(),
            ConnectorConfig::Specified(connector) => Ok(connector),
        }
    }
}

/// Configuration for an `Api`.
#[derive(Debug)]
pub struct Config {
    token: String,
    connector: ConnectorConfig,
}

impl Config {
    /// Set connector type for an `Api`.
    pub fn connector(self, connector: Box<dyn Connector>) -> Config {
        Config {
            token: self.token,
            connector: ConnectorConfig::new(connector),
        }
    }

    /// Create new `Api` instance.
    pub fn build(self) -> Result<Api, Error> {
        Ok(Api(
            Arc::new(ApiInner {
                token: self.token,
                connector: self.connector.take()?,
            }),
        ))
    }
}

impl Api {
    /// Start construction of the `Api` instance.
    ///
    /// # Examples
    ///
    /// Using default connector.
    ///
    /// ```rust
    /// # extern crate telegram_bot;
    /// # extern crate tokio;
    /// use telegram_bot::Api;
    ///
    /// # fn main() {
    /// # let telegram_token = "token";
    /// let api = Api::configure(telegram_token).build().unwrap();
    /// # }
    /// ```
    ///
    /// Using custom connector.
    ///
    ///
    /// ```rust
    /// # extern crate telegram_bot;
    /// # extern crate tokio;
    /// # #[cfg(feature = "hyper_connector")]
    /// # fn main() {
    /// use telegram_bot::Api;
    /// use telegram_bot::connector::hyper;
    ///
    /// # let telegram_token = "token";
    /// let api = Api::configure(telegram_token)
    ///     .connector(hyper::default_connector().unwrap())
    ///     .build().unwrap();
    /// # }
    ///
    /// # #[cfg(not(feature = "hyper_connector"))]
    /// # fn main() {}
    /// ```
    pub fn configure<T: AsRef<str>>(token: T) -> Config {
        Config {
            token: token.as_ref().to_string(),
            connector: Default::default(),
        }
    }

    /// Create a stream which produces updates from the Telegram server.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio;
    /// # use telegram_bot::Api;
    /// # fn main() {
    /// # let api: Api = Api::configure("token").build().unwrap();
    /// use futures::Stream;
    ///
    /// let future = api.stream().for_each(|update| {
    ///     println!("{:?}", update);
    ///     Ok(())
    /// });
    /// # }
    /// ```
    pub fn stream(&self) -> UpdatesStream {
        UpdatesStream::new(self.clone())
    }

    /// Send a request to the Telegram server and wait for a response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio;
    /// # use futures::Future;
    /// # use telegram_bot::{Api, GetMe, ChatId};
    /// # use telegram_bot::prelude::*;
    /// #
    /// # fn main() {
    /// # let telegram_token = "token";
    /// # let api = Api::configure(telegram_token).build().unwrap();
    /// # if false {
    /// let chat = ChatId::new(61031);
    /// api.run(chat.text("Message"))
    /// # }
    /// # }
    // pub async fn send<Req: Request>(&self, request: Req) -> () {
    //     self.send(request).await;
    // }

    /// Send a request to the Telegram server and wait for a response, timing out after `duration`.
    /// Future will resolve to `None` if timeout fired.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio;
    /// # use futures::Future;
    /// # use telegram_bot::{Api, GetMe};
    /// #
    /// # fn main() {
    /// # let telegram_token = "token";
    /// # let api = Api::configure(telegram_token).build().unwrap();
    /// # if false {
    /// use std::time::Duration;
    ///
    /// let future = api.send_timeout(GetMe, Duration::from_secs(5));
    /// future.and_then(|me| Ok(assert!(me.is_some())));
    /// # }
    /// # }
    /// ```
    pub fn send_timeout<Req: Request>(
        &self,
        request: Req,
        duration: Duration,
    ) -> impl Future<Output = Result<Option<<Req::Response as ResponseType>::Type>, Error>> + Send {
        let api = self.clone();
        let request = request.serialize();
        async move {
            match tokio::time::timeout(duration, api.send_http_request::<Req::Response>(request?)).await {
                Err(_) => Ok(None),
                Ok(Ok(result)) => Ok(Some(result)),
                Ok(Err(error)) => Err(error),
            }
        }
    }

    /// Send a request to the Telegram server and wait for a response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio;
    /// # use futures::Future;
    /// # use telegram_bot::{Api, GetMe};
    /// #
    /// # fn main() {
    /// # let telegram_token = "token";
    /// # let api = Api::configure(telegram_token).build().unwrap();
    /// # if false {
    /// let future = api.send(GetMe);
    /// future.and_then(|me| Ok(println!("{:?}", me)));
    /// # }
    /// # }
    /// ```

    pub async fn send<Req: Request + Send>(
        &self,
        request: Req,
    ) -> Result<<Req::Response as ResponseType>::Type, Error> {
        let request = request.serialize();
        let api = self.clone();
        let request = request?;
        let ref token = api.0.token;
        let response = api.0.connector.request(token, request);

        let response = response.await?;
        Req::Response::deserialize(response).map_err(From::from)
    }

    async fn send_http_request<Resp: ResponseType>(
        &self,
        request: HttpRequest,
    ) -> Result<Resp::Type, Error> {
        let ref token = self.0.token;
        let response = self.0.connector.request(token, request);

        let response = response.await?;
        Resp::deserialize(response).map_err(From::from)
    }
}
