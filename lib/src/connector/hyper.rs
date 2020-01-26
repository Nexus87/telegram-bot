//! Connector with hyper backend.

use futures::{Future, FutureExt};
use std::fmt;
use std::str::FromStr;

use hyper::client::Client;
use hyper::header;
use hyper::http::Request;
use hyper::{Method, Uri};
use hyper_tls::HttpsConnector;

use telegram_bot_raw::{Body as TelegramBody, HttpRequest, HttpResponse, Method as TelegramMethod};

use crate::errors::Error;

use super::_base::Connector;
use hyper::client::connect::Connect;
use hyper::Body;
use std::pin::Pin;
/// This connector uses `hyper` backend.
pub struct HyperConnector<C> {
    inner: Client<C>
}

impl<C> fmt::Debug for HyperConnector<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "hyper connector")
    }
}

impl<C> HyperConnector<C> {
    pub fn new(client: Client<C>) -> Self {
        HyperConnector {
            inner: client
        }
    }
}

impl<C: Connect + Sync + Send + Clone + 'static> Connector for HyperConnector<C> {
    fn request(
        &self,
        token: &str,
        req: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, Error>> + Send>> {
        let uri = Uri::from_str(&req.url.url(token));
        let client = self.inner.clone();
        let future = async move {
            let uri = uri?;
            let method = match req.method {
                TelegramMethod::Get => Method::GET,
                TelegramMethod::Post => Method::POST,
            };
            let builder = Request::builder();
            let builder = builder.method(method).uri(uri);

            let http_request = match req.body {
                TelegramBody::Empty => builder.body(Body::empty()).unwrap(),
                TelegramBody::Json(body) => builder
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
                body => panic!("Unknown body type {:?}", body),
            };

            let response = client.request(http_request).await?;

            let body = hyper::body::to_bytes(response.into_body())
                .await
                .iter()
                .fold(vec![], |mut result, chunk| -> Vec<u8> {
                    result.extend_from_slice(&chunk);
                    result
                });
            Ok(HttpResponse { body: Some(body) })
        };

        future.boxed()
    }
}

/// Returns default hyper connector. Uses one resolve thread and `HttpsConnector`.
pub fn default_connector() -> Result<Box<dyn Connector>, Error> {
    let connector = HttpsConnector::new();
    let config = Client::builder();
    Ok(Box::new(HyperConnector::new(config.build(connector))))
}
