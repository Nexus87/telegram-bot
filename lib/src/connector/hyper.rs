//! Connector with hyper backend.

use std::fmt;
use std::str::FromStr;

use futures::{Future, Stream};
use futures::future::result;
use hyper::header;
use hyper::{Method, Uri};
use hyper::http::Request;
use hyper::client::Client;
use hyper_tls::HttpsConnector;

use telegram_bot_raw::{HttpRequest, HttpResponse, Method as TelegramMethod, Body as TelegramBody};

use errors::Error;
use future::{TelegramFuture, NewTelegramFuture};

use super::_base::Connector;
use hyper::Body;
use hyper::client::connect::Connect;
use std::sync::Arc;

/// This connector uses `hyper` backend.
pub struct HyperConnector<C> {
    inner: Arc<Client<C>>
}

impl<C> fmt::Debug for HyperConnector<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "hyper connector")
    }
}

impl<C> HyperConnector<C> {
    pub fn new(client: Client<C>) -> Self {
        HyperConnector {
            inner: Arc::new(client)
        }
    }
}

impl<C: Connect + 'static> Connector for HyperConnector<C> {
    fn request(&self, token: &str, req: HttpRequest) -> TelegramFuture<HttpResponse> {
        let uri = result(Uri::from_str(&req.url.url(token))).map_err(From::from);

        let client = self.inner.clone();
        let request = uri.and_then(move |uri| {
            let method = match req.method {
                TelegramMethod::Get => Method::GET,
                TelegramMethod::Post => Method::POST,
            };
            let mut builder = Request::builder();
            let builder = builder.method(method).uri(uri);

            let http_request = match req.body {
                TelegramBody::Empty => builder.body(Body::empty()).unwrap(),
                TelegramBody::Json(body) => {
                    builder.header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(body))
                        .unwrap()
                }
                body => panic!("Unknown body type {:?}", body)
            };

            client.request(http_request).map_err(From::from)
        });

        let future = request.and_then(move |response| {
            response.into_body().map_err(From::from)
                .fold(vec![], |mut result, chunk| -> Result<Vec<u8>, Error> {
                    result.extend_from_slice(&chunk);
                    Ok(result)
                })
        });

        let future = future.and_then(|body| {
            Ok(HttpResponse {
                body: Some(body),
            })
        });

        TelegramFuture::new(Box::new(future))
    }
}

/// Returns default hyper connector. Uses one resolve thread and `HttpsConnector`.
pub fn default_connector() -> Result<Box<Connector>, Error> {
    let connector = HttpsConnector::new(1).map_err(|err| {
        ::std::io::Error::new(::std::io::ErrorKind::Other, format!("tls error: {}", err))
    })?;
    let config = Client::builder();
    Ok(Box::new(HyperConnector::new(config.build(connector))))
}
