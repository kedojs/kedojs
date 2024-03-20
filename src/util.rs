use boa_engine::{
  js_string,
  object::{
    builtins::{JsFunction, JsPromise},
    FunctionObjectBuilder,
  },
  Context, JsResult, JsValue, NativeFunction,
};
use std::io;
use tokio::task::spawn_blocking;

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
  // SAFETY: `File` doesn't contain types that need tracing.
  unsafe {
    NativeFunction::from_closure(move |this, args, context| {
      let future = f(this, args, context);
      Ok(JsPromise::from_future(future, context).into())
    })
  }
}

pub fn promise_method(
  f: fn(&JsValue, &[JsValue], &mut Context) -> JsPromise,
) -> NativeFunction {
  // SAFETY: `File` doesn't contain types that need tracing.
  unsafe {
    NativeFunction::from_closure(move |this, args, context| {
      Ok(f(this, args, context).into())
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