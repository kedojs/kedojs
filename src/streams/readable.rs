use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use boa_engine::builtins::iterable::create_iter_result_object;
use boa_engine::job::NativeJob;
use boa_engine::object::builtins::{JsPromise, JsUint8Array};
use boa_engine::object::ObjectInitializer;
use boa_engine::property::{Attribute, PropertyDescriptor};
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsSymbol, JsValue};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::StreamExt;
use hyper::body::{Body, Incoming};
use tokio::sync::Mutex;

use crate::errors::KedoError;
use crate::util::{
  create_readable_stream_result, js_function, method_with_state,
  promise_method_with_state,
};

#[derive(Debug)]
pub struct ReadableBytesStream(Incoming);

impl Stream for ReadableBytesStream {
  type Item = Result<Bytes, KedoError>;

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
        std::task::Poll::Ready(Some(Err(e))) => {
          std::task::Poll::Ready(Some(Err(e.into())))
        }
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
  pub locked: bool,
}

impl ReadableStream {
  pub fn new(stream: ReadableBytesStream) -> Self {
    Self {
      stream: Some(stream),
      locked: false,
    }
  }

  #[allow(dead_code)]
  pub fn is_locked(&self) -> bool {
    self.locked
  }

  pub fn aquire_lock(&mut self) -> Result<ReadableBytesStream, JsNativeError> {
    self.locked = true;

    if self.stream.is_none() {
      return Err(
        JsNativeError::typ().with_message("ReadableStream has already been consumed"),
      );
    }

    Ok(self.stream.take().unwrap())
  }

  pub fn to_object(
    rs: Rc<RefCell<ReadableStream>>,
    ctx: &mut Context,
  ) -> JsResult<JsValue> {
    let get_reader = method_with_state(Self::get_reader, rs.clone());
    let get_iterator = method_with_state(Self::get_iterator, rs.clone());
    let iterator_fn = js_function(
      ctx,
      get_iterator,
      &JsSymbol::async_iterator().to_string(),
      0,
    );
    let get_locked = method_with_state(Self::get_locked, rs.clone());
    let get_lcoked = js_function(ctx, get_locked, "locked", 0);
    let object = ObjectInitializer::new(ctx)
      .function(get_reader, js_string!("getReader"), 0)
      .accessor(
        js_string!("locked"),
        Some(get_lcoked),
        None,
        Attribute::READONLY | Attribute::ENUMERABLE,
      )
      .build();

    object.define_property_or_throw(
      JsSymbol::async_iterator(),
      PropertyDescriptor::builder()
        .value(iterator_fn)
        .writable(false)
        .enumerable(true)
        .configurable(false),
      ctx,
    )?;
    Ok(object.into())
  }

  pub fn get_locked(
    _this: &JsValue,
    _args: &[JsValue],
    rs: &mut Self,
    _ctx: &mut Context,
  ) -> JsResult<JsValue> {
    Ok(rs.locked.into())
  }

  pub fn get_iterator(
    _this: &JsValue,
    _args: &[JsValue],
    rs: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue> {
    if rs.locked {
      return Err(
        JsNativeError::typ()
          .with_message("ReadableStream is locked")
          .into(),
      );
    }

    let stream = rs.stream.take().unwrap();
    let stream = Arc::new(Mutex::new(stream));
    let reader = ReadableStreamIterator::new(stream.clone());

    rs.locked = true;
    ReadableStreamIterator::to_object(reader, ctx)
  }

  fn get_reader(
    _this: &JsValue,
    _args: &[JsValue],
    rs: &mut Self,
    ctx: &mut Context,
  ) -> JsResult<JsValue> {
    if rs.locked {
      return Err(
        JsNativeError::typ()
          .with_message("ReadableStream is locked")
          .into(),
      );
    }

    let stream = rs.stream.take().unwrap();
    let stream = Arc::new(Mutex::new(stream));
    let reader = ReadableStreamDefaultReader::new(stream.clone());

    rs.locked = true;
    ReadableStreamDefaultReader::to_object(reader, ctx)
  }
}

pub struct ReadableStreamDefaultReader {
  pub stream: Arc<Mutex<ReadableBytesStream>>,
}

pub trait JsReadableStreamDefaultReader {
  fn read(
    this: &JsValue,
    args: &[JsValue],
    rsd: &mut Self,
    ctx: &mut Context,
  ) -> JsPromise;
  fn release_lock(
    this: &JsValue,
    args: &[JsValue],
    rsd: &mut Self,
    ctx: &mut Context,
  ) -> JsPromise;
  fn cancel(
    this: &JsValue,
    args: &[JsValue],
    rsd: &mut Self,
    ctx: &mut Context,
  ) -> JsPromise;
  fn closed(
    this: &JsValue,
    args: &[JsValue],
    rsd: &mut Self,
    ctx: &mut Context,
  ) -> JsPromise;
}

impl JsReadableStreamDefaultReader for ReadableStreamDefaultReader {
  fn read(
    _this: &JsValue,
    _args: &[JsValue],
    rsd: &mut Self,
    ctx: &mut Context,
  ) -> JsPromise {
    let stream = rsd.stream.clone();

    let (promise, resolvers) = JsPromise::new_pending(ctx);

    let future = async move {
      let mut body = Vec::new();
      // futures_util::pin_mut!(stream); // Pin the stream in place

      // Consume the stream.
      let mut stream_lock = stream.lock().await;

      let mut done = false;
      if let Some(chunk) = stream_lock.next().await {
        if let Ok(data) = chunk {
          body.extend_from_slice(&data);
        }
      } else {
        done = true;
      }

      NativeJob::new(move |context| {
        let chunk = JsUint8Array::from_iter(body, context)?;
        let result = create_readable_stream_result(context, chunk.into(), done);
        resolvers
          .resolve
          .call(&JsValue::undefined(), &[result.into()], context)
      })
    };

    ctx.job_queue().enqueue_future_job(Box::pin(future), ctx);

    promise
  }

  fn release_lock(
    _this: &JsValue,
    _args: &[JsValue],
    _rsd: &mut Self,
    _ctx: &mut Context,
  ) -> JsPromise {
    todo!()
  }

  fn cancel(
    _this: &JsValue,
    _args: &[JsValue],
    _rsd: &mut Self,
    _ctx: &mut Context,
  ) -> JsPromise {
    todo!()
  }

  fn closed(
    _this: &JsValue,
    _args: &[JsValue],
    _rsd: &mut Self,
    _ctx: &mut Context,
  ) -> JsPromise {
    todo!()
  }
}

impl ReadableStreamDefaultReader {
  pub fn new(stream: Arc<Mutex<ReadableBytesStream>>) -> Self {
    Self { stream }
  }

  pub fn to_object(
    rsd: ReadableStreamDefaultReader,
    ctx: &mut boa_engine::Context,
  ) -> JsResult<JsValue> {
    let state = Rc::new(RefCell::new(rsd));
    let object = ObjectInitializer::new(ctx)
      .function(
        promise_method_with_state(Self::read, state.clone()),
        js_string!("read"),
        0,
      )
      .function(
        promise_method_with_state(Self::release_lock, state.clone()),
        js_string!("releaseLock"),
        0,
      )
      .function(
        promise_method_with_state(Self::cancel, state.clone()),
        js_string!("cancel"),
        0,
      )
      .function(
        promise_method_with_state(Self::closed, state.clone()),
        js_string!("closed"),
        0,
      )
      .build();

    Ok(object.into())
  }
}

pub struct ReadableStreamIterator {
  pub stream: Arc<Mutex<ReadableBytesStream>>,
}

impl ReadableStreamIterator {
  pub fn new(stream: Arc<Mutex<ReadableBytesStream>>) -> Self {
    Self { stream }
  }

  fn to_object(iter: Self, context: &mut Context) -> JsResult<JsValue> {
    let state = Rc::new(RefCell::new(iter));
    let next_function = promise_method_with_state(Self::next, state.clone());
    let object = ObjectInitializer::new(context)
      .function(next_function, js_string!("next"), 0)
      .build();

    Ok(object.into())
  }

  fn next(
    _this: &JsValue,
    _: &[JsValue],
    stream: &mut ReadableStreamIterator,
    context: &mut Context,
  ) -> JsPromise {
    let stream = stream.stream.clone();

    let (promise, resolvers) = JsPromise::new_pending(context);

    let future = async move {
      let mut body = Vec::new();
      // futures_util::pin_mut!(stream); // Pin the stream in place

      // Consume the stream.
      let mut stream_lock = stream.lock().await;

      let mut done = false;
      if let Some(chunk) = stream_lock.next().await {
        if let Ok(data) = chunk {
          body.extend_from_slice(&data);
        }
      } else {
        done = true;
      }

      NativeJob::new(move |context| {
        let chunk = JsUint8Array::from_iter(body, context)?;
        let result = create_iter_result_object(chunk.into(), done, context);
        resolvers
          .resolve
          .call(&JsValue::undefined(), &[result.into()], context)
      })
    };

    context
      .job_queue()
      .enqueue_future_job(Box::pin(future), context);

    promise
  }
}
