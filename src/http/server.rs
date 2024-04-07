use std::pin::Pin;

use boa_engine::JsObject;
use bytes::Bytes;
use futures_util::Future;
use http_body_util::Full;
use hyper::{body::Incoming as IncomingBody, service::Service, Request, Response};

#[derive(Debug, Clone)]
pub struct HttpService {
  callback: JsObject,
}

impl Service<Request<IncomingBody>> for HttpService {
  type Response = Response<Full<Bytes>>;
  type Error = hyper::Error;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

  fn call(&self, _req: Request<IncomingBody>) -> Self::Future {
    fn mk_response(s: String) -> Result<Response<Full<Bytes>>, hyper::Error> {
      Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
    }

    Box::pin(async { mk_response(format!("Hello World")) })
  }
}
