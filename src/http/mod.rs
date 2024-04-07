use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use boa_engine::job::NativeJob;
use boa_engine::object::builtins::JsPromise;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::PropertyDescriptor;
use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};
use http_body_util::Full;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{body::Bytes, Request};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

pub mod headers;
mod request;
mod response;
mod server;

use crate::streams::readable::{ReadableBytesStream, ReadableStream};
use crate::util::{js_function, promise_method};

use self::headers::Headers;
use self::request::WebRequest;
use self::response::FetchResponse;

pub async fn fetch_json_evt(
  request: WebRequest,
) -> Result<FetchResponse, Box<dyn std::error::Error>> {
  // let url_str = url.to_string();
  // Parse our URL...
  // let url = url.parse::<hyper::Uri>()?;

  // Get the host and the port
  let host = request.uri.host().unwrap();
  let port = request.uri.port_u16().unwrap_or(80);

  let address = format!("{}:{}", host, port);

  // Open a TCP connection to the remote host
  let stream = TcpStream::connect(address).await?;

  // Use an adapter to access something implementing `tokio::io` traits as if they implement
  // `hyper::rt` IO traits.
  let io = TokioIo::new(stream);

  // Create the Hyper client
  let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

  // Spawn a task to poll the connection, driving the HTTP state
  tokio::task::spawn(async move {
    if let Err(err) = conn.await {
      println!("Connection failed: {:?}", err);
    }
  });

  // The authority of our URL will be the hostname of the httpbin remote
  let authority = request.uri.authority().unwrap().clone();

  // Create an HTTP request
  let body = match request.body {
    Some(body) => {
      let str_body = serde_json::to_string(&body).unwrap();
      Full::new(Bytes::from(str_body))
    }
    None => Full::new(Bytes::new()), // Use Full::new with an empty Bytes instance
  };

  let mut req = Request::builder()
    .uri(request.uri.clone())
    .method(hyper::Method::from_str(request.method.as_str()).unwrap())
    .header(hyper::header::HOST, authority.as_str())
    .body(body)?;

  let headers = req.headers_mut();
  request.headers.iter().for_each(|(key, value)| {
    headers.append(
      HeaderName::from_str(key).unwrap(),
      HeaderValue::from_str(value).unwrap(),
    );
  });

  // Await the response...
  let res = sender.send_request(req).await?;
  let status = res.status().as_u16();
  let ok = res.status().is_success();
  let headers = Headers::from(res.headers().clone());
  let status_text = res.status().canonical_reason().unwrap_or("").to_string();

  let readable_stream = ReadableBytesStream::new(res.into_body());
  let response = FetchResponse::new(
    Rc::new(RefCell::new(ReadableStream::new(readable_stream))),
    status,
    ok,
    headers,
    status_text,
    request.uri.to_string(),
  );

  Ok(response)
}

#[allow(dead_code)]
pub fn init(context: &mut Context) -> JsObject {
  ObjectInitializer::new(context)
    .function(promise_method(fetch_json), js_string!("fetch"), 1)
    .build()
}

pub fn init_with_object(context: &mut Context, object: &JsObject) -> JsResult<bool> {
  let function_fetch_json = js_function(context, promise_method(fetch_json), "fetch", 1);

  object.define_property_or_throw(
    js_string!("fetch"),
    PropertyDescriptor::builder()
      .value(function_fetch_json)
      .writable(true)
      .enumerable(false)
      .configurable(true),
    context,
  )
}

fn fetch_json(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsPromise {
  let url = args
    .get(0)
    .expect("No url argument provided")
    .to_string(context)
    .unwrap()
    .to_std_string_escaped();

  let options = match args.get(1) {
    Some(options) => options.clone(),
    None => JsValue::Object(JsObject::default()),
  };

  let (promise, resolvers) = JsPromise::new_pending(context);

  let uri = url.parse::<hyper::Uri>().unwrap();
  let fetch_task = fetch_json_evt(WebRequest::from_value(options, uri, context).unwrap());

  let future = async move {
    let result = fetch_task.await;

    NativeJob::new(move |context| match result {
      Ok(response) => {
        let res = FetchResponse::to_object(response, context)?;
        resolvers
          .resolve
          .call(&JsValue::undefined(), &[res], context)
      }
      Err(_e) => resolvers.reject.call(&JsValue::undefined(), &[], context),
    })
  };

  context
    .job_queue()
    .enqueue_future_job(Box::pin(future), context);

  promise
}
