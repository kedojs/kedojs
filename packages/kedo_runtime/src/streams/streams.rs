use kedo_core::{
    define_exports, downcast_state, enqueue_job, native_job, ClassTable, ProtoTable,
};
use kedo_utils::{downcast_ref, drop_ptr, js_error, js_error_typ, js_undefined};
use rust_jsc::{
    callback, class::ClassError, constructor, finalize, JSClass, JSClassAttribute,
    JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue, PrivateData,
};

use std::cell::RefCell;
use std::convert::Infallible;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::vec;

use futures::Stream;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StreamError {
    Closed,
    ChannelFull,
    SendError(String),
    ReceiverTaken,
    Empty,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::Closed => write!(f, "Stream is closed"),
            StreamError::ChannelFull => write!(f, "Channel is full"),
            StreamError::ReceiverTaken => write!(f, "Receiver is taken"),
            StreamError::SendError(s) => write!(f, "Send error: {}", s),
            StreamError::Empty => write!(f, "Stream is empty"),
        }
    }
}

pub struct BoundedBufferChannel<T> {
    sender: Option<tokio::sync::mpsc::Sender<T>>,
    receiver: Option<tokio::sync::mpsc::Receiver<T>>,
}

impl<T> Drop for BoundedBufferChannel<T> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<T> BoundedBufferChannel<T> {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity);

        Self {
            sender: Some(sender),
            receiver: Some(receiver),
        }
    }

    pub fn try_write(&self, item: T) -> Result<(), StreamError> {
        self.sender
            .as_ref()
            .ok_or(StreamError::Closed)?
            .try_send(item)
            .map_err(|e| match e {
                tokio::sync::mpsc::error::TrySendError::Full(_) => {
                    StreamError::ChannelFull
                }
                tokio::sync::mpsc::error::TrySendError::Closed(_) => StreamError::Closed,
            })
    }

    pub async fn write(&mut self, item: T) -> Result<(), StreamError> {
        self.sender
            .as_ref()
            .ok_or(StreamError::Closed)?
            .send(item)
            .await
            .map_err(|_| StreamError::Closed)
    }

    pub fn try_read(&mut self) -> Result<T, StreamError> {
        match self.receiver.as_mut() {
            Some(receiver) => receiver.try_recv().map_err(|e| match e {
                tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
                tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                    StreamError::Closed
                }
            }),
            None => Err(StreamError::ReceiverTaken),
        }
    }

    pub async fn read(&mut self) -> Result<T, StreamError> {
        let receiver = self.receiver.as_mut().ok_or(StreamError::ReceiverTaken)?;

        tokio::select! {
            biased;
            msg = receiver.recv() => msg.ok_or(StreamError::Closed),
        }
    }

    pub fn new_reader(&mut self) -> Option<BoundedBufferChannelReader<T>> {
        if let Some(receiver) = self.receiver.take() {
            Some(BoundedBufferChannelReader { receiver })
        } else {
            None
        }
    }

    pub fn new_writer(&self) -> Option<BoundedBufferChannelWriter<T>> {
        Some(BoundedBufferChannelWriter {
            sender: self.sender.as_ref()?.clone(),
        })
    }

    pub fn close(&mut self) {
        let _ = self.receiver.take();
        let _ = self.sender.take();
    }
}

pub struct BoundedBufferChannelWriter<T> {
    sender: tokio::sync::mpsc::Sender<T>,
}

impl<T> BoundedBufferChannelWriter<T> {
    pub async fn write(&self, item: T) -> Result<(), StreamError> {
        self.sender
            .send(item)
            .await
            .map_err(|_| StreamError::Closed)
    }
}

#[derive(Debug)]
pub struct BoundedBufferChannelReader<T> {
    receiver: tokio::sync::mpsc::Receiver<T>,
}

impl<T> BoundedBufferChannelReader<T> {
    pub fn try_read(&mut self) -> Result<T, StreamError> {
        self.receiver.try_recv().map_err(|e| match e {
            tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
            tokio::sync::mpsc::error::TryRecvError::Disconnected => StreamError::Closed,
        })
    }

    pub async fn read(&mut self) -> Result<T, StreamError> {
        tokio::select! {
            biased;
            msg = self.receiver.recv() => msg.ok_or(StreamError::Closed),
        }
    }
}

impl<T> Stream for BoundedBufferChannelReader<T> {
    type Item = Result<T, Infallible>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Safety: We're not moving the struct; we're only accessing its fields.
        let self_mut = unsafe { self.get_unchecked_mut() };

        // Pin the receiver since `poll_recv` requires a `Pin<&mut Receiver<T>>`.
        let mut receiver = Pin::new(&mut self_mut.receiver);

        // Poll the receiver for the next item.
        match receiver.poll_recv(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(Ok(item))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct InternalStreamResource<T> {
    channel: BoundedBufferChannel<T>,
    completion: StreamCompletion,
}

#[derive(Debug, Clone, Default)]
pub struct StreamCompletion {
    inner: std::rc::Rc<RefCell<StreamCompletionInner>>,
}

#[derive(Debug, Default)]
pub struct StreamCompletionInner {
    closed: bool,
    waker: Option<std::task::Waker>,
}

#[derive(Debug)]
pub struct InternalStreamResourceReader<T> {
    reader: BoundedBufferChannelReader<T>,
}

impl<T> InternalStreamResource<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            channel: BoundedBufferChannel::new(capacity),
            completion: StreamCompletion::default(),
        }
    }

    pub fn write(&self, item: T) -> Result<(), StreamError> {
        self.channel.try_write(item)
    }

    pub async fn write_async(&mut self, item: T) -> Result<(), StreamError> {
        self.channel.write(item).await
    }

    pub fn read(&mut self) -> Result<T, StreamError> {
        self.channel.try_read()
    }

    pub async fn read_async(&mut self) -> Result<T, StreamError> {
        self.channel.read().await
    }

    pub fn new_reader(&mut self) -> Option<InternalStreamResourceReader<T>> {
        self.channel
            .new_reader()
            .map(|reader| InternalStreamResourceReader { reader })
    }

    pub fn close(&mut self) {
        self.channel.close();
        self.completion.close();
    }

    pub fn completion(&self) -> StreamCompletion {
        return self.completion.clone();
    }

    pub async fn wait_close(&mut self) -> Result<(), StreamError> {
        self.completion.clone().await
    }
}

impl StreamCompletion {
    pub fn close(&mut self) {
        let mut mut_ref = self.inner.borrow_mut();
        mut_ref.closed = true;
        if let Some(waker) = mut_ref.waker.take() {
            waker.wake();
        }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.borrow_mut().closed
    }

    pub fn set_waker(&mut self, waker: std::task::Waker) {
        self.inner.borrow_mut().waker = Some(waker);
    }
}

impl Future for StreamCompletion {
    type Output = Result<(), StreamError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_closed() {
            Poll::Ready(Ok(()))
        } else {
            let self_mut = unsafe { self.get_unchecked_mut() };
            self_mut.set_waker(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T> InternalStreamResourceReader<T> {
    pub fn read(&mut self) -> Result<T, StreamError> {
        self.reader.try_read()
    }

    pub async fn read_async(&mut self) -> Result<T, StreamError> {
        self.reader.read().await
    }

    pub fn take(self) -> BoundedBufferChannelReader<T> {
        self.reader
    }
}

impl<T> Stream for InternalStreamResourceReader<T> {
    type Item = Result<T, Infallible>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Safety: We're not moving the struct; we're only accessing its fields.
        let self_mut = unsafe { self.get_unchecked_mut() };

        // Pin the receiver since `poll_recv` requires a `Pin<&mut Receiver<T>>`.
        let mut receiver = Pin::new(&mut self_mut.reader.receiver);

        // Poll the receiver for the next item.
        match receiver.poll_recv(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(Ok(item))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for InternalStreamResource<T> {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct JSReadableStreamResource {}

impl JSReadableStreamResource {
    pub const CLASS_NAME: &'static str = "ReadableStreamResource";
    pub const PROTO_NAME: &'static str = "ReadableStreamResourcePrototype";

    pub fn init_proto(
        proto_manager: &mut ProtoTable,
        manager: &mut ClassTable,
        ctx: &JSContext,
    ) -> Result<(), ClassError> {
        let class = manager
            .get(JSReadableStreamResource::CLASS_NAME)
            .expect("ReadableStreamResource class not found");

        let object = class.object::<InternalStreamResource<Vec<u8>>>(ctx, None);
        proto_manager.insert(JSReadableStreamResource::PROTO_NAME.to_string(), object);

        Ok(())
    }

    pub fn template_object(ctx: &JSContext, scope: &JSObject) -> JSResult<()> {
        let state = downcast_state(ctx);
        let template_object = state
            .protos()
            .get(JSReadableStreamResource::PROTO_NAME)
            .expect("ReadableStreamResource prototype not found");

        scope.set_property(
            JSReadableStreamResource::CLASS_NAME,
            &template_object,
            Default::default(),
        )?;
        Ok(())
    }

    pub fn init_class(manaager: &mut ClassTable) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .call_as_constructor(Some(Self::constructor))
            .set_finalize(Some(Self::finalize))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        manaager.insert(class);
        Ok(())
    }

    /// finalize is called when the object is being garbage collected.
    /// This is the place to clean up any resources that the object may hold.
    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<InternalStreamResource<Vec<u8>>>(data_ptr);
    }

    #[constructor]
    fn constructor(
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
            .get(JSReadableStreamResource::CLASS_NAME)
            .expect("ReadableStreamResource class not found");

        let stream = InternalStreamResource::new(high_water_mark as usize);
        let object =
            class.object::<InternalStreamResource<Vec<u8>>>(&ctx, Some(Box::new(stream)));

        object.set_prototype(&constructor);
        Ok(object.into())
    }
}

pub struct JSReadableStreamResourceReader {}

impl JSReadableStreamResourceReader {
    pub const CLASS_NAME: &'static str = "ReadableStreamResourceReader";
    // pub const PROTO_NAME: &'static str = "ReadableStreamResourceReaderPrototype";

    pub fn init_class(manaager: &mut ClassTable) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .call_as_constructor(Some(Self::constructor))
            .set_finalize(Some(Self::finalize))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        manaager.insert(class);
        Ok(())
    }

    /// finalize is called when the object is being garbage collected.
    /// This is the place to clean up any resources that the object may hold.
    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<BoundedBufferChannelReader<Vec<u8>>>(data_ptr);
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        constructor: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = downcast_state(&ctx);
        let class = state
            .classes()
            .get(JSReadableStreamResourceReader::CLASS_NAME)
            .expect("ReadableStreamResourceReader class not found");

        let object = class.object::<BoundedBufferChannelReader<Vec<u8>>>(&ctx, None);
        object.set_prototype(&constructor);
        Ok(object.into())
    }
}

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
    let internal_stream = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource);
    let mut stream = match internal_stream {
        Some(stream) => stream,
        None => return Ok(JSValue::undefined(&ctx)),
    };

    stream.close();
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
    let mut stream = match downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource) {
        Some(stream) => stream,
        None => return Err(js_error_typ!(&ctx, "[Op][Acquire] Invalid stream resource")),
    };

    let reader = match stream.channel.new_reader() {
        Some(reader) => reader,
        None => return Err(js_error_typ!(&ctx, "[Op][Acquire] Reader already taken")),
    };

    let state = downcast_state(&ctx);
    let class = state
        .classes()
        .get(JSReadableStreamResourceReader::CLASS_NAME)
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
    resource: JSObject,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();
    let reader = downcast_ref::<BoundedBufferChannelReader<Vec<u8>>>(&resource);
    let mut stream_reader = match reader {
        Some(reader) => reader,
        None => return Err(js_error_typ!(&ctx, "[Op] Invalid resource reader")),
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
    resource: JSObject,
) -> JSResult<JSValue> {
    let resource_reader = downcast_ref::<BoundedBufferChannelReader<Vec<u8>>>(&resource);
    let mut readable_stream = match resource_reader {
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
    let resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource);
    let stream_resource = match resource {
        Some(stream) => stream,
        None => return Err(js_error_typ!(&ctx, "[Op:WriteSync] Invalid stream")),
    };

    let typed_array = JSTypedArray::from_value(&chunk)?;
    let bytes = typed_array.as_vec::<u8>()?;
    let len = bytes.len() as f64;

    match stream_resource.write(bytes) {
        Ok(_) => Ok(JSValue::number(&ctx, len).into()),
        Err(e) => Err(js_error_typ!(&ctx, e.to_string())),
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
    let stream_resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource);
    let stream_resource = match stream_resource {
        Some(stream) => stream,
        None => return Err(JSError::new_typ(&ctx, "[Op:Write]Invalid stream resource")?),
    };

    let stream_writer = stream_resource.channel.new_writer();
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
    let stream_resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource);
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
    @template[JSReadableStreamResource],
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_internal_stream_resource() {
        let mut stream = InternalStreamResource::<Vec<u8>>::new(5);
        for i in 0..5 {
            stream.write(vec![i as u8]).unwrap();
        }

        assert_eq!(stream.read().unwrap(), vec![0]);
        assert_eq!(stream.read().unwrap(), vec![1]);
        assert_eq!(stream.read().unwrap(), vec![2]);
        assert_eq!(stream.read().unwrap(), vec![3]);
        assert_eq!(stream.read().unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_internal_stream_resource_async() {
        let mut stream = InternalStreamResource::<Vec<u8>>::new(5);
        for i in 0..5 {
            stream.write_async(vec![i as u8]).await.unwrap();
        }

        assert_eq!(stream.read_async().await.unwrap(), vec![0]);
        assert_eq!(stream.read_async().await.unwrap(), vec![1]);
        assert_eq!(stream.read_async().await.unwrap(), vec![2]);
        assert_eq!(stream.read_async().await.unwrap(), vec![3]);
        assert_eq!(stream.read_async().await.unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_internal_stream_resource_async_close() {
        let mut stream = InternalStreamResource::<Vec<u8>>::new(5);
        for i in 0..5 {
            stream.write_async(vec![i as u8]).await.unwrap();
        }

        let mut stream_reader = stream.channel.new_reader().unwrap();
        stream.close();
        let result = stream_reader.read().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0]);

        drop(stream_reader);
        let result = stream.write_async(vec![5]).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StreamError::Closed);
    }

    #[test]
    fn test_bounded_buffer_channel() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.try_write(vec![i as u8]).unwrap();
        }

        assert_eq!(channel.try_read().unwrap(), vec![0]);
        assert_eq!(channel.try_read().unwrap(), vec![1]);
        assert_eq!(channel.try_read().unwrap(), vec![2]);
        assert_eq!(channel.try_read().unwrap(), vec![3]);
        assert_eq!(channel.try_read().unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_internal_stream_resource_wait_close() {
        let stream = InternalStreamResource::<Vec<u8>>::new(5);
        for i in 0..5 {
            stream.write(vec![i as u8]).unwrap();
        }

        let box_stream = Box::new(stream);
        let raw_ptr = Box::into_raw(box_stream);

        let wait_close_future = async {
            let wait_future = unsafe { (*raw_ptr).wait_close() };
            wait_future.await.unwrap();
        };

        let close_future = async {
            unsafe { (*raw_ptr).close() };
        };

        tokio::join!(wait_close_future, close_future);
    }

    #[tokio::test]
    async fn test_internal_stream_resource_completion() {
        let mut stream = InternalStreamResource::<Vec<u8>>::new(5);
        for i in 0..5 {
            stream.write(vec![i as u8]).unwrap();
        }

        let completion = stream.completion();
        let future = async {
            completion.await.unwrap();
        };

        let close_future = async {
            // wait for 2 seconds before closing the stream
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            stream.close();
        };

        tokio::join!(
            tokio::time::timeout(std::time::Duration::from_secs(2), future),
            close_future
        )
        .0
        .unwrap();
    }

    #[tokio::test]
    async fn test_bounded_buffer_channel_async() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write(vec![i as u8]).await.unwrap();
        }

        assert_eq!(channel.read().await.unwrap(), vec![0]);
        assert_eq!(channel.read().await.unwrap(), vec![1]);
        assert_eq!(channel.read().await.unwrap(), vec![2]);
        assert_eq!(channel.read().await.unwrap(), vec![3]);
        assert_eq!(channel.read().await.unwrap(), vec![4]);
    }

    #[test]
    fn test_bounded_buffer_channel_reader() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.try_write(vec![i as u8]).unwrap();
        }

        let mut reader = channel.new_reader().unwrap();
        assert_eq!(reader.try_read().unwrap(), vec![0]);
        assert_eq!(reader.try_read().unwrap(), vec![1]);
        assert_eq!(reader.try_read().unwrap(), vec![2]);
        assert_eq!(reader.try_read().unwrap(), vec![3]);
        assert_eq!(reader.try_read().unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_bounded_buffer_channel_reader_async() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write(vec![i as u8]).await.unwrap();
        }

        let mut reader = channel.new_reader().unwrap();
        assert_eq!(reader.read().await.unwrap(), vec![0]);
        assert_eq!(reader.read().await.unwrap(), vec![1]);
        assert_eq!(reader.read().await.unwrap(), vec![2]);
        assert_eq!(reader.read().await.unwrap(), vec![3]);
        assert_eq!(reader.read().await.unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_bound_buffer_channel_limit() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.try_write(vec![i as u8]).unwrap();
        }

        let future = async {
            channel.write(vec![5]).await.unwrap();
        };

        let result =
            tokio::time::timeout(std::time::Duration::from_secs(1), future).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_bound_buffer_channel_limit() {
        let channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.try_write(vec![i as u8]).unwrap();
        }

        let result = channel.try_write(vec![5]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StreamError::ChannelFull);
    }
}
