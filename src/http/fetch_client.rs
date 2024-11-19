use rust_jsc::{
    callback, JSContext, JSError, JSFunction, JSObject, JSPromise, JSResult, JSValue,
};

use super::request::FetchRequest;
use crate::{
    context::downcast_state,
    http::fetch_errors::FetchError,
    job::{AsyncJobQueue, NativeJob},
    signals::InternalSignal,
    utils::downcast_ref,
};

#[callback]
fn op_internal_fetch(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let request_args = match args.get(0) {
        Some(arg) => arg,
        None => return Err(JSError::new_typ(&ctx, "Missing arguments")?),
    };
    let request = FetchRequest::from_value(request_args, &ctx)?;
    let signal = request_args.as_object()?.get_property("signal")?;
    let mut internal_signal = None;
    if !signal.is_undefined() && signal.is_object() {
        let signal = signal.as_object()?;
        let oneshot_signal = downcast_ref::<InternalSignal>(&signal)
            .map(|mut signal| signal.as_mut().get_signal());
        if let Some(signal) = oneshot_signal {
            internal_signal = signal;
        }
    }

    let (promise, resolver) = JSPromise::new_pending(&ctx)?;
    let state = downcast_state::<AsyncJobQueue>(&ctx);
    let fetch_client = state.globals();
    let response = fetch_client
        .fetch_client()
        .execute(request)
        .map_err(|err| {
            JSError::new_typ(&ctx, format!("Failed to fetch: {}", err)).unwrap()
        })?;

    let future = async move {
        let result = if let Some(mut signal) = internal_signal {
            tokio::select! {
                res = response => res,
                _ = signal.wait() => {
                    Err(FetchError::new("Fetch aborted"))
                },
            }
        } else {
            response.await
        };
        NativeJob::new(move |ctx| {
            match result {
                Ok(res) => {
                    // resolve or reject the promise
                    let response_value: JSResult<JSValue> = res.to_value(ctx);
                    match response_value {
                        Ok(value) => {
                            resolver.resolve(None, &[value.into()])?;
                        }
                        Err(err) => {
                            let err_value =
                                JSError::with_message(ctx, format!("{}", err)).unwrap();
                            resolver.reject(None, &[err_value.into()])?;
                        }
                    }
                }
                Err(err) => {
                    let err_value =
                        JSError::with_message(ctx, format!("{}", err)).unwrap();
                    resolver.reject(None, &[err_value.into()])?;
                }
            }
            Ok(())
        })
        .set_name("op_internal_fetch")
    };

    downcast_state::<AsyncJobQueue>(&ctx)
        .job_queue()
        .borrow()
        .spawn(Box::pin(future));
    Ok(promise.into())
}

pub fn fetch_exports(ctx: &JSContext, exports: &JSObject) {
    let op_internal_fetch_fn =
        JSFunction::callback(ctx, Some("op_internal_fetch"), Some(op_internal_fetch));

    let op_read_response_stream_fn = JSFunction::callback(
        ctx,
        Some("op_read_response_stream"),
        Some(super::response::op_read_response_stream),
    );

    exports
        .set_property(
            "op_internal_fetch",
            &op_internal_fetch_fn,
            Default::default(),
        )
        .expect("Unable to set fetch property");
    exports
        .set_property(
            "op_read_response_stream",
            &op_read_response_stream_fn,
            Default::default(),
        )
        .expect("Unable to set fetch property");
}
