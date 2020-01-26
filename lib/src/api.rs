use crate::connector::{default_connector, Connector};
use crate::errors::{Error, ErrorKind};
use crate::stream::{NewUpdatesStream, UpdatesStream};
use futures::Future;
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
    /// Create a new `Api` instance.
    ///
    /// # Example
    ///
    /// Using default connector.
    ///
    /// ```
    /// use telegram_bot::Api;
    ///
    /// # fn main() {
    /// # let telegram_token = "token";
    /// let api = Api::new(telegram_token);
    /// # }
    /// ```
    pub fn new<T: AsRef<str>>(token: T) -> Self {
        Self::with_connector(token, default_connector().unwrap())
    }

    /// Create a new `Api` instance wtih custom connector.
    pub fn with_connector<T: AsRef<str>>(token: T, connector: Box<dyn Connector>) -> Self {
        Api(Arc::new(ApiInner {
            token: token.as_ref().to_string(),
            connector,
        }))
    }
    /// Start construction of the `Api` instance.
    ///
    /// # Examples
    ///
    /// Using default connector.
    ///
    /// ```
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
    /// ```
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
    /// 
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
    /// ```
    /// # use telegram_bot::Api;
    /// use futures::StreamExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let api: Api = Api::new("token");
    ///
    /// let mut stream = api.stream();
    /// let update = stream.next().await;
    ///     println!("{:?}", update);
    /// # }
    /// ```
    pub fn stream(&self) -> UpdatesStream {
        UpdatesStream::new(self.clone())
    }

    /// Send a request to the Telegram server and wait for a response, timing out after `duration`.
    /// Future will resolve to `None` if timeout fired.
    ///
    /// # Examples
    ///
    /// ```
    /// # use telegram_bot::{Api, GetMe};
    /// # use std::time::Duration;
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let telegram_token = "token";
    /// # let api = Api::new(telegram_token);
    /// # if false {
    /// let result = api.send_timeout(GetMe, Duration::from_secs(2)).await;
    /// println!("{:?}", result);
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
            match tokio::time::timeout(duration, api.send_http_request::<Req::Response>(request.map_err(ErrorKind::from)?)).await {
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
    /// ```
    /// # use telegram_bot::{Api, GetMe};
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let telegram_token = "token";
    /// # let api = Api::new(telegram_token);
    /// # if false {
    /// let result = api.send(GetMe).await;
    /// println!("{:?}", result);
    /// # }
    /// # }
    /// ```

    pub async fn send<Req: Request + Send>(
        &self,
        request: Req,
    ) -> Result<<Req::Response as ResponseType>::Type, Error> {
        let request = request.serialize();
        let api = self.clone();
        let request = request.map_err(ErrorKind::from)?;
        let ref token = api.0.token;
        let response = api.0.connector.request(token, request);

        let response = response.await?;
        let response = Req::Response::deserialize(response).map_err(ErrorKind::from)?;
        Ok(response)
    }

    async fn send_http_request<Resp: ResponseType>(
        &self,
        request: HttpRequest,
    ) -> Result<Resp::Type, Error> {
        let ref token = self.0.token;
        let response = self.0.connector.request(token, request);

        let response = response.await?;
        let response = Resp::deserialize(response).map_err(ErrorKind::from)?;
        Ok(response)
    }
}
