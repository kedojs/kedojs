use std::cell::RefCell;
use std::rc::Rc;
use std::{future::Future, pin::Pin};

use boa_engine::job::NativeJob;
use boa_engine::object::builtins::JsPromise;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{
  js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction,
};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::StreamExt;
use hyper::body::{Body, Incoming};
use hyper::header;

// use crate::util::async_method_with_state;

use super::headers::Headers;

#[derive(Debug)]
pub struct ReadableBytesStream(Incoming);

impl Stream for ReadableBytesStream {
  type Item = Result<Bytes, hyper::Error>;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    let this = std::pin::Pin::into_inner(self);

    loop {
      break match Pin::new(&mut this.0).poll_frame(cx) {
        std::task::Poll::Ready(Some(Ok(chunk))) => {
          if let Ok(data) = chunk.into_data() {
            if !data.is_empty() {
              break std::task::Poll::Ready(Some(Ok(data)));
            }
          }

          continue;
        }
        std::task::Poll::Ready(Some(Err(e))) => std::task::Poll::Ready(Some(Err(e))),
        std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
        std::task::Poll::Pending => std::task::Poll::Pending,
      };
    }
  }
}

impl ReadableBytesStream {
  pub fn new(incoming: Incoming) -> Self {
    ReadableBytesStream(incoming)
  }
}

// https://streams.spec.whatwg.org/#rs
// #[derive(Debug, Trace, Finalize, JsData)]
pub struct ReadableStream {
  // #[unsafe_ignore_trace]
  stream: Option<ReadableBytesStream>,
}

impl ReadableStream {
  pub fn new(stream: ReadableBytesStream) -> Self {
    Self {
      stream: Some(stream),
    }
  }
}

// https://fetch.spec.whatwg.org/#concept-fetch
// #[derive(Debug, Trace, Finalize, JsData)]
pub struct FetchResponse {
  // #[unsafe_ignore_trace]
  body: ReadableStream,
  body_used: bool,
  status: u16,
  ok: bool,
  headers: Headers,
  status_text: String,
  url: String,
}

trait JsResponse {
  fn to_json(
    this: &JsValue,
    arg: &[JsValue],
    res: &mut Self,
    ctx: &mut Context,
  ) -> JsPromise;
  fn to_json_static(
    this: &JsValue,
    _: &[JsValue],
    res: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue>;
  fn to_text(
    this: &JsValue,
    _: &[JsValue],
    res: &mut Self,
    ctx: &mut Context,
  ) -> impl Future<Output = JsResult<JsValue>>;
  fn to_blob(
    this: &JsValue,
    _: &[JsValue],
    res: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue>;
  fn to_array_buffer(
    this: &JsValue,
    _: &[JsValue],
    res: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue>;
}

impl FetchResponse {
  pub fn new(
    body: ReadableStream,
    status: u16,
    ok: bool,
    headers: Headers,
    status_text: String,
    url: String,
  ) -> Self {
    Self {
      body,
      body_used: false,
      status,
      ok,
      headers,
      status_text,
      url,
    }
  }
}

impl JsResponse for FetchResponse {
  /// https://developer.mozilla.org/en-US/docs/Web/API/Response/json
  ///
  /// Response: json() method
  /// The json() method of the Response interface takes a Response stream and reads it to completion.
  /// It returns a promise which resolves with the result of parsing the body text as JSON.
  /// Note that despite the method being named json(), the result is not JSON but is instead
  /// the result of taking JSON as input and parsing it to produce a JavaScript object.
  fn to_json(
    _this: &JsValue,
    _args: &[JsValue],
    response: &mut FetchResponse,
    ctx: &mut Context,
  ) -> JsPromise {
    let stream = response.body.stream.take().unwrap();

    let (promise, resolvers) = JsPromise::new_pending(ctx);

    let future = async move {
      let mut body = Vec::new();
      futures_util::pin_mut!(stream); // Pin the stream in place

      // Consume the stream.
      while let Some(chunk) = stream.next().await {
        if let Ok(data) = chunk {
          body.extend_from_slice(&data);
        }
      }

      // println!("End Stream: {:?}", response.body.stream.0.is_end_stream());
      NativeJob::new(move |context| {
        let body = String::from_utf8(body)
          .map_err(|_| JsNativeError::typ().with_message("Invalid UTF-8"))?;
        let json = serde_json::from_str(&body)
          .map_err(|_| JsNativeError::typ().with_message("Invalid JSON"))?;
        let json_result = JsValue::from_json(&json, context)?;
        resolvers
          .resolve
          .call(&JsValue::undefined(), &[json_result], context)
      })
    };

    ctx.job_queue().enqueue_future_job(Box::pin(future), ctx);

    promise
  }

  fn to_json_static(
    _this: &JsValue,
    _args: &[JsValue],
    response: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue> {
    todo!()
  }

  fn to_text(
    _this: &JsValue,
    _args: &[JsValue],
    response: &mut Self,
    ctx: &mut Context,
  ) -> impl Future<Output = JsResult<JsValue>> {
    async { todo!() }
  }

  fn to_blob(
    _this: &JsValue,
    _: &[JsValue],
    response: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue> {
    todo!()
  }

  fn to_array_buffer(
    _this: &JsValue,
    _args: &[JsValue],
    response: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue> {
    todo!()
  }
}

impl FetchResponse {
  pub fn to_object(
    response: FetchResponse,
    ctx: &mut boa_engine::Context,
  ) -> JsResult<JsValue> {
    let status = response.status;
    let ok = response.ok;
    let status_text = js_string!(response.status_text.clone());
    let headers = response.headers.clone();
    let url = js_string!(response.url.clone());
    let headers = headers.to_object(ctx)?;

    let state = Rc::new(RefCell::new(response));
    let object = ObjectInitializer::new(ctx)
      .function(
        Self::async_method_with_state(Self::to_json, state.clone()),
        js_string!("json"),
        0,
      )
      .property(
        js_string!("status"),
        JsValue::new(status),
        Attribute::READONLY | Attribute::ENUMERABLE,
      )
      // .accessor(key, get, set, attribute)
      .property(
        js_string!("ok"),
        JsValue::new(ok),
        Attribute::READONLY | Attribute::ENUMERABLE,
      )
      .property(
        js_string!("statusText"),
        status_text,
        Attribute::READONLY | Attribute::ENUMERABLE,
      )
      .property(
        js_string!("url"),
        url,
        Attribute::READONLY | Attribute::ENUMERABLE,
      )
      .property(
        js_string!("headers"),
        JsValue::new(headers),
        Attribute::READONLY | Attribute::ENUMERABLE,
      )
      .build();

    Ok(object.into())
  }

  fn async_method_with_state(
    f: fn(&JsValue, &[JsValue], &mut FetchResponse, &mut Context) -> JsPromise,
    state: Rc<RefCell<FetchResponse>>,
  ) -> NativeFunction {
    // SAFETY: `File` doesn't contain types that need tracing.
    unsafe {
      NativeFunction::from_closure(move |this, args, context| {
        Ok(f(this, args, &mut state.borrow_mut(), context).into())
      })
    }
  }
}

// https://fetch.spec.whatwg.org/#response
