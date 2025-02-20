use crate::http::response::FetchResponseExt;
use crate::stream_codec::op_read_decoded_stream;
use crate::{http::request::FetchRequestExt, signals::InternalSignal};
use kedo_core::{define_exports, downcast_state, enqueue_job, native_job};
use kedo_macros::js_class;
use kedo_std::{FetchClient, FetchError, FetchRequest};
use kedo_utils::{downcast_ref, js_error, js_undefined};
use rust_jsc::{callback, JSContext, JSError, JSObject, JSResult, JSValue};

#[js_class(
    resource = FetchClient,
)]
pub struct FetchClientResource {}

#[callback]
fn op_new_fetch_client(ctx: JSContext, _: JSObject, _: JSObject) -> JSResult<JSValue> {
    let fetch_client = FetchClient::new();
    let binding = downcast_state(&ctx);
    let class = binding
        .classes()
        .get(FetchClientResource::CLASS_NAME)
        .expect("FetchClient class not found");

    let object = class.object(&ctx, Some(Box::new(fetch_client)));
    object.protect();
    Ok(object.into())
}

/// [Op:InternalFetch]
/// Internal fetch operation
/// This operation is used to fetch a resource from the network
/// It takes a client, request and a callback function as arguments
/// The callback function is called with the response or an error
///
/// e.g. op_internal_fetch(client, request, callback)
#[callback]
fn op_internal_fetch(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    client: JSObject,
    request_arg: JSValue,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();
    let request = FetchRequest::from_value(&request_arg, &ctx)?;
    let signal = request_arg.as_object()?.get_property("signal")?;
    let mut internal_signal = None;
    if !signal.is_undefined() && signal.is_object() {
        let signal = signal.as_object()?;
        let oneshot_signal = downcast_ref::<InternalSignal>(&signal)
            .map(|mut signal| signal.as_mut().get_signal());
        if let Some(signal) = oneshot_signal {
            internal_signal = signal;
        }
    }

    let state = downcast_state(&ctx);
    let client = downcast_ref::<FetchClient>(&client);
    let client = match client {
        Some(client) => client,
        None => return Err(JSError::new_typ(&ctx, "[Op:InternalFetch] Invalid client")?),
    };
    let response = match client.execute(request) {
        Ok(response) => response,
        Err(err) => {
            return Err(JSError::new_typ(&ctx, format!("Failed to fetch: {}", err))?)
        }
    };

    enqueue_job!(state, async move {
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

        native_job!("op_internal_fetch", move |ctx| {
            match result {
                Ok(res) => {
                    // resolve or reject the promise
                    let response_value: JSResult<JSValue> = res.to_value(ctx);
                    match response_value {
                        Ok(value) => {
                            callback.call(None, &[js_undefined!(&ctx), value.into()])?;
                        }
                        Err(err) => {
                            let err_value = js_error!(ctx, format!("{}", err));
                            callback.call(None, &[err_value.into()])?;
                        }
                    }
                }
                Err(err) => {
                    let err_value = js_error!(ctx, format!("{}", err));
                    callback.call(None, &[err_value.into()])?;
                }
            }

            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

pub struct FetchModule {}

define_exports!(
    FetchModule,
    @template[],
    @function[
        op_new_fetch_client,
        op_internal_fetch,
        op_read_decoded_stream
    ]
);
