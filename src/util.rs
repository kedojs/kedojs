use boa_engine::{
  js_string,
  object::{
    builtins::{JsFunction, JsPromise},
    FunctionObjectBuilder, ObjectInitializer,
  },
  property::Attribute,
  Context, JsObject, JsResult, JsValue, NativeFunction,
};
use std::{cell::RefCell, io, rc::Rc};
use tokio::task::spawn_blocking;

#[derive(Debug, Clone, Copy)]
pub(crate) enum PropertyNameKind {
  Key,
  Value,
  KeyAndValue,
}

pub async fn asyncify<F, T>(f: F) -> io::Result<T>
where
  F: FnOnce() -> io::Result<T> + Send + 'static,
  T: Send + 'static,
{
  match spawn_blocking(f).await {
    Ok(res) => res,
    Err(_) => Err(io::Error::new(
      io::ErrorKind::Other,
      "background task failed",
    )),
  }
}

pub fn async_method<
  Fut: std::future::IntoFuture<Output = JsResult<JsValue>> + 'static,
>(
  f: fn(&JsValue, &[JsValue], &mut Context) -> Fut,
) -> NativeFunction {
  // SAFETY: it doesn't contain types that need tracing.
  unsafe {
    NativeFunction::from_closure(move |this, args, context| {
      let future = f(this, args, context);
      Ok(JsPromise::from_future(future, context).into())
    })
  }
}

pub fn method_with_state<T: 'static>(
  f: fn(&JsValue, &[JsValue], &mut T, &mut Context) -> JsResult<JsValue>,
  state: Rc<RefCell<T>>,
) -> NativeFunction {
  // SAFETY: it doesn't contain types that need tracing.
  unsafe {
    NativeFunction::from_closure(move |this, args, context| {
      f(this, args, &mut state.borrow_mut(), context)
    })
  }
}

#[allow(dead_code)]
pub fn async_method_with_state<
  Fut: std::future::IntoFuture<Output = JsResult<JsValue>> + 'static,
  T: 'static,
>(
  f: fn(&JsValue, &[JsValue], &mut T, &mut Context) -> Fut,
  state: Rc<RefCell<T>>,
) -> NativeFunction {
  // SAFETY: it doesn't contain types that need tracing.
  unsafe {
    NativeFunction::from_closure(move |this, args, context| {
      let future = f(this, args, &mut state.borrow_mut(), context);
      Ok(JsPromise::from_future(future, context).into())
    })
  }
}

pub fn promise_method(
  f: fn(&JsValue, &[JsValue], &mut Context) -> JsPromise,
) -> NativeFunction {
  // SAFETY: it doesn't contain types that need tracing.
  unsafe {
    NativeFunction::from_closure(move |this, args, context| {
      Ok(f(this, args, context).into())
    })
  }
}

pub fn promise_method_with_state<T: 'static>(
  f: fn(&JsValue, &[JsValue], &mut T, &mut Context) -> JsPromise,
  state: Rc<RefCell<T>>,
) -> NativeFunction {
  // SAFETY: it doesn't contain types that need tracing.
  unsafe {
    NativeFunction::from_closure(move |this, args, context| {
      Ok(f(this, args, &mut state.borrow_mut(), context).into())
    })
  }
}

pub fn js_function(
  context: &mut Context,
  function: NativeFunction,
  name: &str,
  length: usize,
) -> JsFunction {
  FunctionObjectBuilder::new(context.realm(), function)
    .name(js_string!(name))
    .length(length)
    .constructor(false)
    .build()
}

pub fn create_readable_stream_result(
  context: &mut Context,
  value: JsValue,
  done: bool,
) -> JsObject {
  ObjectInitializer::new(context)
    .property(
      js_string!("value"),
      value,
      Attribute::READONLY | Attribute::ENUMERABLE,
    )
    .property(
      js_string!("done"),
      done,
      Attribute::READONLY | Attribute::ENUMERABLE,
    )
    .build()
}
