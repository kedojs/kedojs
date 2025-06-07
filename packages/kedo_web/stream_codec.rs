use futures::StreamExt;
use kedo_core::{downcast_state, enqueue_job, native_job};
use kedo_macros::js_class;
use kedo_std::{StreamDecoder, StreamEncoder};
use kedo_utils::{downcast_ref, js_error, js_error_typ, js_undefined};
use rust_jsc::{callback, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue};
use std::mem::ManuallyDrop;

/// | ---------------------- DecodedStreamResource ---------------------- |
///
/// This class is used to create a JS object that wraps a decoded stream.
/// The object is used to read the decoded stream in chunks.
#[js_class(
    resource = StreamDecoder,
)]
pub struct DecodedStreamResource {}

#[callback]
pub fn op_read_decoded_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    resource: JSObject,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();
    let state = downcast_state(&ctx);
    let resource = downcast_ref::<StreamDecoder>(&resource);
    let mut decoded_stream = match resource {
        Some(resource) => resource,
        None => {
            return Err(js_error_typ!(
                &ctx,
                "[Op:ReadDecoded] Invalid internal resource"
            ))
        }
    };

    enqueue_job!(state, async move {
        let result = decoded_stream.next().await;
        native_job!("op_read_decoded_stream", move |ctx| {
            match result {
                Some(Ok(bytes)) => {
                    let mut bytes: ManuallyDrop<Vec<u8>> =
                        ManuallyDrop::new(bytes.to_vec());
                    let chunk = JSTypedArray::with_bytes(
                        ctx,
                        bytes.as_mut_slice(),
                        rust_jsc::JSTypedArrayType::Uint8Array,
                    )?
                    .into();
                    callback.call(None, &[js_undefined!(&ctx), chunk])?;
                }
                Some(Err(err)) => {
                    let error = js_error!(ctx, err.message);
                    callback.call(None, &[error.into()])?;
                }
                None => {
                    callback.call(None, &[])?;
                }
            }

            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

/// | ---------------------- EncodedStreamResource ---------------------- |
///
/// This class is used to create a JS object that wraps an encoded stream.
/// The object is used to read the encoded stream in chunks.
#[js_class(
    resource = StreamEncoder,
    proto = "EncodedStreamResourcePrototype",
)]
pub struct EncodedStreamResource {}
