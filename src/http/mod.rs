use async_stream::stream;
use boa_engine::job::NativeJob;
use boa_engine::object::builtins::JsPromise;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::PropertyDescriptor;
use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};
use http_body_util::{BodyExt, Empty};
use hyper::{body::Bytes, Request};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

pub mod headers;
mod response;
mod server;

use crate::util::{js_function, promise_method};

use self::headers::Headers;
use self::response::{FetchResponse, ReadableBytesStream, ReadableStream};

pub async fn fetch_json_evt(
  url: &str,
) -> Result<FetchResponse, Box<dyn std::error::Error>> {
  let url_str = url.to_string();
  // Parse our URL...
  let url = url.parse::<hyper::Uri>()?;

  // Get the host and the port
  let host = url.host().unwrap();
  let port = url.port_u16().unwrap_or(80);

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
  let authority = url.authority().unwrap().clone();

  // Create an HTTP request with an empty body and a HOST header
  let req = Request::builder()
    .uri(url)
    .method(hyper::Method::GET)
    .header(hyper::header::HOST, authority.as_str())
    .body(Empty::<Bytes>::new())?;

  // Await the response...
  let res = sender.send_request(req).await?;
  let status = res.status().as_u16();
  let ok = res.status().is_success();
  let headers = Headers::from(res.headers().clone());
  let status_text = res.status().canonical_reason().unwrap_or("").to_string();
  // let url = res.

  // res.collect()
  let readable_stream = ReadableBytesStream::new(res.into_body());
  let response = FetchResponse::new(
    ReadableStream::new(readable_stream),
    status,
    ok,
    headers,
    status_text,
    url_str,
  );
  Ok(response)
  // asynchronously aggregate the chunks of the body
  // let body = res.collect().await?.aggregate();
  // try to parse as json with serde_json
  // let res_json = serde_json::from_reader(body.reader())?;
  // Ok(res_json)
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

  let (promise, resolvers) = JsPromise::new_pending(context);

  let future = async move {
    let result = fetch_json_evt(&url).await;

    NativeJob::new(move |context| match result {
      Ok(response) => {
        // let json = JsValue::from_json(&entries, context).unwrap();

        let res = FetchResponse::to_object(response, context)?;
        resolvers
          .resolve
          .call(&JsValue::undefined(), &[res], context)
        // todo!("Implement resolve")
      }
      Err(_e) => resolvers.reject.call(&JsValue::undefined(), &[], context),
    })
  };

  context
    .job_queue()
    .enqueue_future_job(Box::pin(future), context);

  promise
}
