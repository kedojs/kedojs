use kedo_core::{define_exports, downcast_state, enqueue_job, native_job};
use kedo_macros::js_class;
use kedo_std::{BoundedBufferChannel, BoundedBufferChannelReader, StreamError};
use kedo_utils::{downcast_ref, js_error, js_error_typ, js_undefined};
use rust_jsc::{
    callback, constructor, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue,
};
use std::mem::ManuallyDrop;
use std::vec;

#[js_class(
    resource = BoundedBufferChannel<Vec<u8>>,
    proto = "ReadableStreamResourcePrototype",
    constructor = stream_reource_constructor,
)]
pub struct ReadableStreamResource {}

#[constructor]
fn stream_reource_constructor(
    ctx: JSContext,
    constructor: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let high_water_mark = match args.get(0) {
        Some(arg) => arg.as_number()?,
        None => 64.0,
    };

    let state = downcast_state(&ctx);
    let class = state
        .classes()
        .get(ReadableStreamResource::CLASS_NAME)
        .expect("ReadableStreamResource class not found");

    let stream = BoundedBufferChannel::new(high_water_mark as usize);
    let object =
        class.object::<BoundedBufferChannel<Vec<u8>>>(&ctx, Some(Box::new(stream)));

    object.set_prototype(&constructor);
    Ok(object.into())
}

#[js_class(
    resource = BoundedBufferChannelReader<Vec<u8>>,
)]
pub struct ReadableStreamResourceReader {}

/// | ---------------------- Stream Resource API ---------------------- |
/// |                                                                   |
/// | 1. op_close_stream_resource                                       |
/// | 2. op_acquire_stream_reader                                       |
/// | 3. op_read_readable_stream                                        |
/// | 4. op_read_sync_readable_stream                                   |
/// | 5. op_write_readable_stream                                       |
/// | 6. op_write_sync_readable_stream                                  |
/// | 7. op_wait_close_readable_stream                                  |
/// |                                                                   |
/// | ------------------------------------------------------------------|

/// [callback] op_close_stream_resource
/// Close the stream resource
/// This is called when the stream is being closed
/// e.g:
/// op_close_stream_resource(rid);
#[callback]
pub fn op_close_stream_resource(
    ctx: JSContext,
    _: JSObject,
    __: JSObject,
    resource: JSObject,
) -> JSResult<JSValue> {
    let internal_stream = downcast_ref::<BoundedBufferChannel<Vec<u8>>>(&resource);
    let mut channel = match internal_stream {
        Some(stream) => stream,
        None => return Ok(JSValue::undefined(&ctx)),
    };

    channel.close();
    Ok(JSValue::undefined(&ctx))
}

/// [callback] op_acquire_stream_reader
/// Acquire a reader for the stream resource
/// e.g:
/// const reader = op_acquire_stream_reader(resource);
#[callback]
pub fn op_acquire_stream_reader(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    resource: JSObject,
) -> JSResult<JSValue> {
    let mut channel = match downcast_ref::<BoundedBufferChannel<Vec<u8>>>(&resource) {
        Some(channel) => channel,
        None => return Err(js_error_typ!(&ctx, "[Op][Acquire] Invalid stream resource")),
    };

    let reader = match channel.aquire_reader() {
        Some(reader) => reader,
        None => return Err(js_error_typ!(&ctx, "[Op][Acquire] Reader already taken")),
    };

    let state = downcast_state(&ctx);
    let class = state
        .classes()
        .get(ReadableStreamResourceReader::CLASS_NAME)
        .expect("ReadableStreamResourceReader class not found");

    let stream_reader =
        class.object::<BoundedBufferChannelReader<Vec<u8>>>(&ctx, Some(Box::new(reader)));

    Ok(stream_reader.into())
}

/// [callback] op_read_readable_stream
/// Read from the stream resource
/// e.g:
/// op_read_readable_stream(reader, callback);
#[callback]
pub fn op_read_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    reader: JSObject,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();
    let reader = downcast_ref::<BoundedBufferChannelReader<Vec<u8>>>(&reader);
    let mut stream_reader = match reader {
        Some(reader) => reader,
        None => return Err(js_error_typ!(&ctx, "[Op:Read] Invalid reader")),
    };

    let state = downcast_state(&ctx);
    enqueue_job!(state, async move {
        let result = stream_reader.read().await;
        native_job!("op_read_readable_stream", move |ctx| {
            match result {
                Ok(bytes) => {
                    let mut bytes = ManuallyDrop::new(bytes);
                    let chunk = JSTypedArray::with_bytes(
                        ctx,
                        bytes.as_mut_slice(),
                        rust_jsc::JSTypedArrayType::Uint8Array,
                    )?
                    .into();
                    callback.call(None, &[js_undefined!(&ctx), chunk])?;
                }
                Err(err) => match err {
                    StreamError::Closed => {
                        callback.call(None, &[js_undefined!(&ctx)])?;
                    }
                    _ => {
                        let error = js_error!(ctx, format!("{}", err));
                        callback.call(None, &[error.into()])?;
                    }
                },
            }

            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

/// [callback] op_read_sync_readable_stream
/// Read synchronously from the stream resource
/// e.g:
/// const chunk = op_read_sync_readable_stream(reader);
#[callback]
fn op_read_sync_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    reader: JSObject,
) -> JSResult<JSValue> {
    let reader = downcast_ref::<BoundedBufferChannelReader<Vec<u8>>>(&reader);
    let mut readable_stream = match reader {
        Some(stream) => stream,
        None => return Err(js_error_typ!(&ctx, "[Op][ReadSync] Invalid stream reader")),
    };

    match readable_stream.try_read() {
        Ok(bytes) => {
            let mut bytes = ManuallyDrop::new(bytes);
            let typed_array = JSTypedArray::with_bytes(
                &ctx,
                bytes.as_mut_slice(),
                rust_jsc::JSTypedArrayType::Int8Array,
            )?;
            Ok(typed_array.into())
        }
        Err(_) => Ok(js_undefined!(&ctx)),
    }
}

/// [callback] op_write_sync_readable_stream
/// Write synchronously to the stream resource
/// e.g:
/// const len = op_write_sync_readable_stream(resource, chunk);
#[callback]
fn op_write_sync_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    resource: JSObject,
    chunk: JSObject,
) -> JSResult<JSValue> {
    let resource = downcast_ref::<BoundedBufferChannel<Vec<u8>>>(&resource);
    let channel = match resource {
        Some(stream) => stream,
        None => return Err(js_error_typ!(&ctx, "[Op:WriteSync] Invalid stream")),
    };

    let typed_array = JSTypedArray::from(chunk);
    let bytes = typed_array.as_vec::<u8>()?;
    let len = bytes.len() as f64;

    match channel.try_write(bytes) {
        Ok(_) => Ok(JSValue::number(&ctx, len).into()),
        Err(e) => {
            // if stream is closed return -1 otherwise return -2 if the buffer is full
            match e {
                StreamError::Closed => Ok(JSValue::number(&ctx, -1 as f64).into()),
                StreamError::ChannelFull => Ok(JSValue::number(&ctx, -2 as f64).into()),
                _ => Err(js_error_typ!(&ctx, format!("{}", e))),
            }
        }
    }
}

/// [callback] op_write_readable_stream
/// Write to the stream resource
/// e.g:
/// op_write_readable_stream(resource, chunk, callback);
#[callback]
fn op_write_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    resource: JSObject,
    chunk: JSObject,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();
    let channel = downcast_ref::<BoundedBufferChannel<Vec<u8>>>(&resource);
    let channel = match channel {
        Some(stream) => stream,
        None => return Err(JSError::new_typ(&ctx, "[Op:Write]Invalid stream resource")?),
    };

    let stream_writer = channel.writer();
    let stream_writer = match stream_writer {
        Some(writer) => writer,
        None => return Err(JSError::new_typ(&ctx, "[Op:Write] Channel already close")?),
    };

    let typed_array = JSTypedArray::from_value(&chunk)?;
    let bytes = typed_array.as_vec::<u8>()?;
    let len = bytes.len() as f64;

    let state = downcast_state(&ctx);
    enqueue_job!(state, async move {
        let result = stream_writer.write(bytes).await;
        native_job!("op_write_readable_stream", move |ctx| {
            match result {
                Ok(_) => {
                    callback
                        .call(None, &[js_undefined!(&ctx), JSValue::number(ctx, len)])?;
                }
                Err(err) => match err {
                    StreamError::Closed => {
                        callback.call(
                            None,
                            &[js_undefined!(&ctx), JSValue::number(ctx, -1 as f64)],
                        )?;
                    }
                    _ => {
                        let error = js_error!(ctx, format!("{}", err));
                        callback.call(None, &[error.into()])?;
                    }
                },
            }

            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

/// [callback] op_wait_close_readable_stream
/// Wait for the stream resource to close
/// e.g:
/// op_wait_close_readable_stream(resource, should_block, callback);
#[callback]
fn op_wait_close_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    resource: JSObject,
    should_block: bool,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();
    let stream_resource = downcast_ref::<BoundedBufferChannel<Vec<u8>>>(&resource);
    let stream_resource = match stream_resource {
        Some(stream) => stream,
        None => return Err(js_error_typ!(&ctx, "[Op:WaitClose] Invalid stream")),
    };

    let completion = stream_resource.completion();
    let future = async move {
        let result = completion.await;
        native_job!("op_wait_close_readable_stream", move |ctx| {
            match result {
                Ok(_) => {
                    callback.call(None, &[])?;
                }
                Err(err) => {
                    let err_value = js_error!(ctx, format!("{}", err));
                    callback.call(None, &[err_value.into()])?;
                }
            }

            callback.unprotect();
            Ok(())
        })
    };

    let state = downcast_state(&ctx);
    let queue = state.job_queue().borrow();
    match should_block {
        true => queue.spawn(Box::pin(future)),
        false => queue.spawn_non_blocking(Box::pin(future)),
    };

    Ok(js_undefined!(&ctx))
}

pub struct StreamResourceModule {}

define_exports!(
    StreamResourceModule,
    @template[ReadableStreamResource],
    @function[
        op_read_readable_stream,
        op_read_sync_readable_stream,
        op_write_readable_stream,
        op_write_sync_readable_stream,
        op_close_stream_resource,
        op_wait_close_readable_stream,
        op_acquire_stream_reader,
    ]
);
