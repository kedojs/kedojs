use kedo_core::{define_exports, downcast_state, enqueue_job, native_job};
use kedo_macros::js_class;
use kedo_std::{
    BoundedBufferChannel, BoundedBufferChannelReader, BufferChannel, BufferChannelReader,
    BufferChannelWriter, StreamError, UnboundedBufferChannel,
    UnboundedBufferChannelReader,
};
use kedo_utils::{downcast_ref, js_error, js_error_typ, js_undefined};
use rust_jsc::{
    callback, constructor, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue,
};
use std::mem::ManuallyDrop;
use std::vec;

/// | ---------------------- Bounded Stream Resource ---------------------- |
/// |                                                                       |
/// | 1. ReadableStreamResource                                             |
/// | 2. ReadableStreamResourceReader                                       |
/// | --------------------------------------------------------------------- |
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

/// | ----------------------- Unbounded Stream Resource --------------------- |
/// |                                                                         |
/// | 1. UnboundedReadableStreamResource                                      |
/// | 2. UnboundedReadableStreamResourceReader                                |
/// | ----------------------------------------------------------------------- |
#[js_class(
    resource = UnboundedBufferChannel<Vec<u8>>,
    proto = "UnboundedStreamResourcePrototype",
    constructor = unbounded_stream_resource_constructor,
)]
pub struct UnboundedReadableStreamResource {}

#[constructor]
fn unbounded_stream_resource_constructor(
    ctx: JSContext,
    constructor: JSObject,
    _: &[JSValue],
) -> JSResult<JSValue> {
    let state = downcast_state(&ctx);
    let class = state
        .classes()
        .get(UnboundedReadableStreamResource::CLASS_NAME)
        .expect("UnboundedReadableStreamResource class not found");

    let stream = UnboundedBufferChannel::new();
    let object =
        class.object::<UnboundedBufferChannel<Vec<u8>>>(&ctx, Some(Box::new(stream)));

    object.set_prototype(&constructor);
    Ok(object.into())
}

#[js_class(
    resource = UnboundedBufferChannelReader<Vec<u8>>,
)]
pub struct UnboundedReadableStreamResourceReader {}

fn bytes_to_js_value(ctx: &JSContext, bytes: Vec<u8>) -> JSResult<JSValue> {
    let mut bytes = ManuallyDrop::new(bytes);
    let chunk = JSTypedArray::with_bytes(
        ctx,
        bytes.as_mut_slice(),
        rust_jsc::JSTypedArrayType::Uint8Array,
    )?
    .into();
    Ok(chunk)
}

/// read macro
macro_rules! channel_op_read {
    ($name:ident, $stream:ident) => {
        #[callback]
        fn $name(
            ctx: JSContext,
            _: JSObject,
            _: JSObject,
            reader: JSObject,
            callback: JSObject,
        ) -> JSResult<JSValue> {
            callback.protect();
            let reader = downcast_ref::<$stream<Vec<u8>>>(&reader);
            let mut stream_reader = match reader {
                Some(reader) => reader,
                None => return Err(js_error_typ!(&ctx, "[Op:Read] Invalid reader")),
            };

            let state = downcast_state(&ctx);
            enqueue_job!(state, async move {
                let result = stream_reader.read().await;
                native_job!(stringify!($name), move |ctx| {
                    match result {
                        Ok(bytes) => {
                            let chunk = bytes_to_js_value(&ctx, bytes)?;
                            callback.call(None, &[js_undefined!(&ctx), chunk])?;
                        }
                        Err(err) => match err {
                            StreamError::Closed => {
                                callback.call(
                                    None,
                                    &[
                                        js_undefined!(&ctx),
                                        JSValue::number(&ctx, err.into()),
                                    ],
                                )?;
                            }
                            StreamError::Empty => {
                                callback.call(
                                    None,
                                    &[
                                        js_undefined!(&ctx),
                                        JSValue::number(&ctx, err.into()),
                                    ],
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
    };
}

macro_rules! channel_op_read_sync {
    ($name:ident, $stream:ident) => {
        #[callback]
        fn $name(
            ctx: JSContext,
            _: JSObject,
            _: JSObject,
            reader: JSObject,
        ) -> JSResult<JSValue> {
            let reader = downcast_ref::<$stream<Vec<u8>>>(&reader);
            let mut readable_stream = match reader {
                Some(stream) => stream,
                None => {
                    return Err(js_error_typ!(
                        &ctx,
                        "[Op][ReadSync] Invalid stream reader"
                    ))
                }
            };

            match readable_stream.try_read() {
                Ok(bytes) => {
                    let chunk = bytes_to_js_value(&ctx, bytes)?;
                    Ok(chunk)
                }
                Err(err) => match err {
                    StreamError::Closed => Ok(JSValue::number(&ctx, err.into())),
                    StreamError::Empty => Ok(JSValue::number(&ctx, err.into())),
                    _ => Err(js_error!(&ctx, format!("{}", err))),
                },
            }
        }
    };
}

macro_rules! channel_op_write {
    ($name:ident, $stream:ident) => {
        #[callback]
        fn $name(
            ctx: JSContext,
            _: JSObject,
            _: JSObject,
            resource: JSObject,
            chunk: JSValue,
            callback: JSObject,
        ) -> JSResult<JSValue> {
            callback.protect();
            let channel = downcast_ref::<$stream<Vec<u8>>>(&resource);
            let mut channel = match channel {
                Some(stream) => stream,
                None => {
                    return Err(js_error_typ!(&ctx, "[Op:Write]Invalid stream resource"))
                }
            };

            let stream_writer = channel.acquire_writer();
            let stream_writer = match stream_writer {
                Some(writer) => writer,
                None => {
                    return Err(js_error_typ!(&ctx, "[Op:Write] Channel already close"))
                }
            };

            let typed_array = JSTypedArray::from_value(&chunk)?;
            let bytes = typed_array.as_vec::<u8>()?;
            let len = bytes.len() as f64;

            let state = downcast_state(&ctx);
            enqueue_job!(state, async move {
                let result = stream_writer.write(bytes).await;
                native_job!(stringify!($name), move |ctx| {
                    match result {
                        Ok(_) => {
                            callback.call(
                                None,
                                &[js_undefined!(&ctx), JSValue::number(&ctx, len)],
                            )?;
                        }
                        Err(err) => match err {
                            StreamError::Closed => {
                                callback.call(
                                    None,
                                    &[
                                        js_undefined!(&ctx),
                                        JSValue::number(&ctx, -1 as f64),
                                    ],
                                )?;
                            }
                            _ => {
                                let error = js_error!(&ctx, format!("{}", err));
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
    };
}

macro_rules! channel_op_write_sync {
    ($name:ident, $stream:ident) => {
        #[callback]
        fn $name(
            ctx: JSContext,
            _: JSObject,
            _: JSObject,
            resource: JSObject,
            chunk: JSValue,
        ) -> JSResult<JSValue> {
            let resource = downcast_ref::<$stream<Vec<u8>>>(&resource);
            let channel = match resource {
                Some(stream) => stream,
                None => return Err(js_error_typ!(&ctx, "[Op:WriteSync] Invalid stream")),
            };

            let bytes = JSTypedArray::bytes_from_value(&chunk)?.to_vec();
            let len = bytes.len() as f64;

            match channel.writer().unwrap().try_write(bytes) {
                Ok(_) => Ok(JSValue::number(&ctx, len).into()),
                Err(e) => {
                    // if stream is closed return -1 otherwise return -2 if the buffer is full
                    match e {
                        StreamError::Closed => {
                            Ok(JSValue::number(&ctx, -1 as f64).into())
                        }
                        StreamError::ChannelFull => {
                            Ok(JSValue::number(&ctx, -2 as f64).into())
                        }
                        _ => Err(js_error_typ!(&ctx, format!("{}", e))),
                    }
                }
            }
        }
    };
}

macro_rules! channel_op_wait_close {
    ($name:ident, $stream:ident) => {
        #[callback]
        fn $name(
            ctx: JSContext,
            _: JSObject,
            _: JSObject,
            resource: JSObject,
            should_block: bool,
            callback: JSObject,
        ) -> JSResult<JSValue> {
            callback.protect();
            let stream_resource = downcast_ref::<$stream<Vec<u8>>>(&resource);
            let stream_resource = match stream_resource {
                Some(stream) => stream,
                None => return Err(js_error_typ!(&ctx, "[Op:WaitClose] Invalid stream")),
            };

            let completion = stream_resource.completion();
            let future = async move {
                let result = completion.await;
                native_job!(stringify!($name), move |ctx| {
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
    };
}

macro_rules! channel_op_close {
    ($name:ident, $stream:ident) => {
        #[callback]
        fn $name(
            ctx: JSContext,
            _: JSObject,
            _: JSObject,
            resource: JSObject,
        ) -> JSResult<JSValue> {
            let internal_stream = downcast_ref::<$stream<Vec<u8>>>(&resource);
            let mut channel = match internal_stream {
                Some(stream) => stream,
                None => return Ok(JSValue::undefined(&ctx)),
            };

            channel.close();
            Ok(JSValue::undefined(&ctx))
        }
    };
}

macro_rules! channel_op_acquire_reader {
    ($name:ident, $stream:ident, $class_name:ident, $reader:ident) => {
        #[callback]
        fn $name(
            ctx: JSContext,
            _: JSObject,
            _: JSObject,
            resource: JSObject,
        ) -> JSResult<JSValue> {
            let mut channel = match downcast_ref::<$stream<Vec<u8>>>(&resource) {
                Some(channel) => channel,
                None => {
                    return Err(js_error_typ!(
                        &ctx,
                        "[Op][Acquire] Invalid stream resource"
                    ))
                }
            };

            let reader = match channel.acquire_reader() {
                Some(reader) => reader,
                None => {
                    return Err(js_error_typ!(&ctx, "[Op][Acquire] Reader already taken"))
                }
            };

            let state = downcast_state(&ctx);
            let class = state
                .classes()
                .get($class_name::CLASS_NAME)
                .expect("ReadableStreamResourceReader class not found");

            let stream_reader =
                class.object::<$reader<Vec<u8>>>(&ctx, Some(Box::new(reader)));

            Ok(stream_reader.into())
        }
    };
}

// | --------------------- Unbounded Stream Resource API --------------------- |
// |                                                                           |
// | 1. op_close_unbounded_stream                                              |
// | 2. op_acquire_unbounded_stream_reader                                     |
// | 3. op_read_unbounded_stream                                               |
// | 4. op_read_sync_unbounded_stream                                          |
// | 5. op_write_unbounded_stream                                              |
// | 6. op_write_sync_unbounded_stream                                         |
// | 7. op_wait_close_unbounded_stream                                         |
// |                                                                           |
// | ------------------------------------------------------------------------- |
channel_op_read!(op_read_unbounded_stream, UnboundedBufferChannelReader);
channel_op_read_sync!(op_read_sync_unbounded_stream, UnboundedBufferChannelReader);
channel_op_write!(op_write_unbounded_stream, UnboundedBufferChannel);
channel_op_write_sync!(op_write_sync_unbounded_stream, UnboundedBufferChannel);
channel_op_close!(op_close_unbounded_stream, UnboundedBufferChannel);
channel_op_wait_close!(op_wait_close_unbounded_stream, UnboundedBufferChannel);
channel_op_acquire_reader!(
    op_acquire_unbounded_stream_reader,
    UnboundedBufferChannel,
    UnboundedReadableStreamResourceReader,
    UnboundedBufferChannelReader
);

// | ---------------------- Stream Resource API ---------------------- |
// |                                                                   |
// | 1. op_close_stream_resource                                       |
// | 2. op_acquire_stream_reader                                       |
// | 3. op_read_readable_stream                                        |
// | 4. op_read_sync_readable_stream                                   |
// | 5. op_write_readable_stream                                       |
// | 6. op_write_sync_readable_stream                                  |
// | 7. op_wait_close_readable_stream                                  |
// |                                                                   |
// | ------------------------------------------------------------------|
channel_op_read!(op_read_readable_stream, BoundedBufferChannelReader);
channel_op_read_sync!(op_read_sync_readable_stream, BoundedBufferChannelReader);
channel_op_write!(op_write_readable_stream, BoundedBufferChannel);
channel_op_write_sync!(op_write_sync_readable_stream, BoundedBufferChannel);
channel_op_close!(op_close_stream_resource, BoundedBufferChannel);
channel_op_wait_close!(op_wait_close_readable_stream, BoundedBufferChannel);
channel_op_acquire_reader!(
    op_acquire_stream_reader,
    BoundedBufferChannel,
    ReadableStreamResourceReader,
    BoundedBufferChannelReader
);

/// | --------------------- Stream Resource Module ---------------------- |
/// | This module provides utilities for working with stream resources.   |
/// | 1. StreamResourceModule                                             |
/// | 2. ReadableStreamResource                                           |
/// | 3. ReadableStreamResourceReader                                     |
/// | 4. UnboundedReadableStreamResource                                  |
/// |                                                                     |
/// | ------------------------------------------------------------------- |
pub struct StreamResourceModule {}

define_exports!(
    StreamResourceModule,
    @template[ReadableStreamResource, UnboundedReadableStreamResource],
    @function[
        op_read_readable_stream,
        op_read_sync_readable_stream,
        op_write_readable_stream,
        op_write_sync_readable_stream,
        op_close_stream_resource,
        op_wait_close_readable_stream,
        op_acquire_stream_reader,

        op_read_unbounded_stream,
        op_read_sync_unbounded_stream,
        op_write_unbounded_stream,
        op_write_sync_unbounded_stream,
        op_close_unbounded_stream,
        op_wait_close_unbounded_stream,
        op_acquire_unbounded_stream_reader,
    ]
);
