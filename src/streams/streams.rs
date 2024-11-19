use rust_jsc::class::ClassError;
use rust_jsc::{
    callback, constructor, finalize, JSClass, JSClassAttribute, JSContext, JSError,
    JSFunction, JSObject, JSPromise, JSResult, JSTypedArray, JSValue, PrivateData,
};

use std::cell::RefCell;
use std::convert::Infallible;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use futures::Stream;

use crate::class_table::ClassTable;
use crate::context::downcast_state;
use crate::job::{AsyncJobQueue, NativeJob};
use crate::proto_table::ProtoTable;
use crate::utils::{downcast_ref, drop_ptr};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StreamError {
    Closed,
    ChannelFull,
    SendError(String),
    Empty,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::Closed => write!(f, "Stream is closed"),
            StreamError::ChannelFull => write!(f, "Channel is full"),
            StreamError::SendError(s) => write!(f, "Send error: {}", s),
            StreamError::Empty => write!(f, "Stream is empty"),
        }
    }
}

pub struct BoundedBufferChannel<T> {
    sender: Option<Sender<T>>,
    receiver: Option<Receiver<T>>,
    is_closed: Arc<AtomicBool>,
}

impl<T> BoundedBufferChannel<T> {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = channel(capacity);
        let is_closed = Arc::new(AtomicBool::new(false));

        Self {
            is_closed,
            sender: Some(sender),
            receiver: Some(receiver),
        }
    }

    pub fn write(&self, item: T) -> Result<(), StreamError> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(StreamError::Closed);
        }

        self.sender
            .as_ref()
            .unwrap()
            .try_send(item)
            .map_err(|e| match e {
                tokio::sync::mpsc::error::TrySendError::Full(_) => {
                    StreamError::ChannelFull
                }
                _ => StreamError::SendError(e.to_string()),
            })
    }

    pub async fn write_async(&mut self, item: T) -> Result<(), StreamError> {
        if self.is_closed.load(Ordering::Relaxed) || self.sender.is_none() {
            return Err(StreamError::Closed);
        }

        self.sender
            .as_ref()
            .unwrap()
            .send(item)
            .await
            .map_err(|e| StreamError::SendError(e.to_string()))
    }

    pub fn read(&mut self) -> Result<T, StreamError> {
        // if self.is_closed.load(Ordering::Relaxed) {
        //     return Err(StreamError::Closed);
        // }

        // self.receiver.try_recv().map_err(|e| match e {
        //     tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
        //     tokio::sync::mpsc::error::TryRecvError::Disconnected => StreamError::Closed,
        // })
        match self.receiver.as_mut() {
            Some(receiver) => receiver.try_recv().map_err(|e| match e {
                tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
                tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                    StreamError::Closed
                }
            }),
            None => Err(StreamError::Closed),
        }
    }

    pub async fn read_async(&mut self) -> Result<Option<T>, StreamError> {
        // if self.is_closed.load(Ordering::Relaxed) {
        //     return Err(StreamError::Closed);
        // }

        let receiver = self.receiver.as_mut().ok_or(StreamError::Closed)?;
        if self.is_closed.load(Ordering::Relaxed) && receiver.is_empty() {
            return Ok(None);
        }

        tokio::select! {
            biased;
            msg = receiver.recv() => Ok(msg),
            // self.receiver.recv() => Ok(msg)
            // _ = &mut self.close_receiver => {
            //     self.close_receiver.close();
            //     Err(StreamError::Closed)
            // },
        }
    }

    pub fn new_reader(&mut self) -> Option<BoundedBufferChannelReader<T>> {
        if let Some(receiver) = self.receiver.take() {
            Some(BoundedBufferChannelReader { receiver })
        } else {
            None
        }
    }

    pub fn close(&mut self) {
        self.is_closed.store(true, Ordering::Relaxed);
        self.sender.take();
    }
}

#[derive(Debug)]
pub struct BoundedBufferChannelReader<T> {
    receiver: Receiver<T>,
}

impl<T> BoundedBufferChannelReader<T> {
    pub fn read(&mut self) -> Result<T, StreamError> {
        self.receiver.try_recv().map_err(|e| match e {
            tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
            tokio::sync::mpsc::error::TryRecvError::Disconnected => StreamError::Closed,
        })
    }

    pub async fn read_async(&mut self) -> Result<Option<T>, StreamError> {
        tokio::select! {
            biased;
            msg = self.receiver.recv() => Ok(msg),
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
    handle: StreamCompletion,
}

#[derive(Debug, Clone, Default)]
pub struct StreamCompletion {
    inner: std::rc::Rc<RefCell<StreamCompletionInner>>,
}

#[derive(Debug, Clone, Default)]
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
            handle: StreamCompletion::default(),
        }
    }

    pub fn write(&self, item: T) -> Result<(), StreamError> {
        self.channel.write(item)
    }

    pub async fn write_async(&mut self, item: T) -> Result<(), StreamError> {
        self.channel.write_async(item).await
    }

    pub fn read(&mut self) -> Result<T, StreamError> {
        self.channel.read()
    }

    pub async fn read_async(&mut self) -> Result<Option<T>, StreamError> {
        self.channel.read_async().await
    }

    pub fn new_reader(&mut self) -> Option<InternalStreamResourceReader<T>> {
        self.channel
            .new_reader()
            .map(|reader| InternalStreamResourceReader { reader })
    }

    pub fn close(&mut self) {
        self.channel.close();
        self.handle.close();
    }

    // pub fn completion(&self) -> StreamCompletion {
    //     return self.handle.clone();
    // }

    pub async fn wait_close(&mut self) -> Result<(), StreamError> {
        self.handle.clone().await
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
        self.reader.read()
    }

    pub async fn read_async(&mut self) -> Result<Option<T>, StreamError> {
        self.reader.read_async().await
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
        let class = manager.get(JSReadableStreamResource::CLASS_NAME).unwrap();
        let template_object = class.object::<InternalStreamResource<Vec<u8>>>(ctx, None);
        proto_manager.insert(
            JSReadableStreamResource::PROTO_NAME.to_string(),
            template_object,
        );

        Ok(())
    }

    pub fn template_object(ctx: &JSContext, scope: &JSObject) -> JSResult<()> {
        let state = downcast_state::<AsyncJobQueue>(ctx);
        let template_object = state
            .protos()
            .get(JSReadableStreamResource::PROTO_NAME)
            .unwrap();
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
        let high_water_mark = args
            .get(0)
            .ok_or_else(|| {
                JSError::new_typ(&ctx, "Missing highWaterMark argument").unwrap()
            })?
            .as_number()?;

        let state = downcast_state::<AsyncJobQueue>(&ctx);
        let class = state
            .classes()
            .get(JSReadableStreamResource::CLASS_NAME)
            .unwrap();

        let stream_resource = InternalStreamResource::new(high_water_mark as usize);
        let object = class.object::<InternalStreamResource<Vec<u8>>>(
            &ctx,
            Some(Box::new(stream_resource)),
        );

        object.set_prototype(&constructor);
        Ok(object.into())
    }
}

#[callback]
pub fn op_read_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let resource_args = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing arguments").unwrap())?
        .as_object()?;
    let state = downcast_state::<AsyncJobQueue>(&ctx);
    let resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource_args);
    let mut readable_stream = resource.ok_or_else(|| {
        JSError::new_typ(&ctx, "Invalid internal resource object").unwrap()
    })?;

    let (promise, resolver) = JSPromise::new_pending(&ctx)?;
    let future = async move {
        let result = readable_stream.read_async().await;
        NativeJob::new(move |ctx| {
            match result {
                Ok(bytes) => {
                    let chunk = if let Some(bytes) = bytes {
                        let mut bytes = ManuallyDrop::new(bytes);
                        JSTypedArray::with_bytes(
                            ctx,
                            bytes.as_mut_slice(),
                            rust_jsc::JSTypedArrayType::Uint8Array,
                        )?
                        .into()
                    } else {
                        JSValue::undefined(ctx)
                    };
                    resolver.resolve(None, &[chunk])?;
                }
                Err(err) => {
                    let err_value =
                        JSError::with_message(ctx, format!("{}", err)).unwrap();
                    resolver.reject(None, &[err_value.into()])?;
                }
            }
            Ok(())
        }).set_name("op_read_readable_stream")
    };

    state.job_queue().borrow().spawn(Box::pin(future));
    Ok(promise.into())
}

#[callback]
fn op_read_sync_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let resource_args = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing arguments").unwrap())?
        .as_object()?;
    let resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource_args);
    let mut readable_stream = resource.ok_or_else(|| {
        JSError::new_typ(&ctx, "Invalid internal resource object").unwrap()
    })?;

    match readable_stream.read() {
        Ok(bytes) => {
            let mut bytes = ManuallyDrop::new(bytes);
            let typed_array = JSTypedArray::with_bytes(
                &ctx,
                bytes.as_mut_slice(),
                rust_jsc::JSTypedArrayType::Int8Array,
            )?;
            Ok(typed_array.into())
        }
        Err(_) => Ok(JSValue::undefined(&ctx)),
    }
}

#[callback]
fn op_write_sync_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let resource_args = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing arguments").unwrap())?
        .as_object()?;
    let resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource_args);
    let readable_stream = resource.ok_or_else(|| {
        JSError::new_typ(&ctx, "Invalid internal resource object").unwrap()
    })?;

    let chunk = args
        .get(1)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing chunk argument").unwrap())?
        .as_object()?;
    let typed_array = JSTypedArray::from_value(&chunk)?;
    let bytes = typed_array.as_vec::<u8>()?;
    let len = bytes.len() as f64;

    match readable_stream.write(bytes) {
        Ok(_) => Ok(JSValue::number(&ctx, len).into()),
        Err(e) => Err(JSError::new_typ(&ctx, e.to_string()).unwrap()),
    }
}

#[callback]
fn op_write_readable_stream(
    ctx: JSContext,
    _: JSObject,
    this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let state = downcast_state::<AsyncJobQueue>(&ctx);
    let resource_args = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing arguments").unwrap())?
        .as_object()?;
    let resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource_args);
    let mut readable_stream = resource.ok_or_else(|| {
        JSError::new_typ(&ctx, "Invalid internal resource object").unwrap()
    })?;

    let chunk = args
        .get(1)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing chunk argument").unwrap())?
        .as_object()?;
    let typed_array = JSTypedArray::from_value(&chunk)?;
    let bytes = typed_array.as_vec::<u8>()?;
    let len = bytes.len() as f64;

    let (promise, resolver) = JSPromise::new_pending(&ctx)?;
    let future = async move {
        let result = readable_stream.write_async(bytes).await;
        NativeJob::new(move |ctx| {
            match result {
                Ok(_) => {
                    resolver.resolve(Some(&this), &[JSValue::number(ctx, len)])?;
                }
                Err(err) => match err {
                    StreamError::Closed => {
                        resolver
                            .resolve(Some(&this), &[JSValue::number(ctx, -1 as f64)])?;
                    }
                    _ => {
                        let err_value =
                            JSError::with_message(ctx, format!("{}", err)).unwrap();
                        resolver.reject(Some(&this), &[err_value.into()])?;
                    }
                },
            }
            Ok(())
        })
        .set_name("op_write_readable_stream")
    };

    state.job_queue().borrow().spawn(Box::pin(future));
    Ok(promise.into())
}

#[callback]
fn op_close_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let resource_args = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing arguments").unwrap())?
        .as_object()?;
    let resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource_args);
    let mut readable_stream = resource.ok_or_else(|| {
        JSError::new_typ(&ctx, "Invalid internal resource object").unwrap()
    })?;

    readable_stream.close();
    Ok(JSValue::undefined(&ctx))
}

#[callback]
fn op_wait_close_readable_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let resource_args = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing arguments").unwrap())?
        .as_object()?;
    let resource = downcast_ref::<InternalStreamResource<Vec<u8>>>(&resource_args);
    let mut readable_stream = resource.ok_or_else(|| {
        JSError::new_typ(&ctx, "Invalid internal resource object").unwrap()
    })?;
    let should_block = args
        .get(1)
        .and_then(|arg| Some(arg.as_boolean()))
        .unwrap_or(true);

    let (promise, resolver) = JSPromise::new_pending(&ctx)?;
    let future = async move {
        let result = readable_stream.wait_close().await;
        NativeJob::new(move |ctx| {
            match result {
                Ok(_) => {
                    resolver.resolve(None, &[])?;
                }
                Err(err) => {
                    let err_value =
                        JSError::with_message(ctx, format!("{}", err)).unwrap();
                    resolver.reject(None, &[err_value.into()])?;
                }
            }
            Ok(())
        }).set_name("op_wait_close_readable_stream")
    };

    let binding = downcast_state::<AsyncJobQueue>(&ctx);
    let queue = binding.job_queue().borrow();
    match should_block {
        true => queue.spawn(Box::pin(future)),
        false => queue.spawn_non_blocking(Box::pin(future)),
    };

    Ok(promise.into())
}

pub fn readable_stream_exports(ctx: &JSContext, exports: &JSObject) {
    let op_read_readable_stream_fn = JSFunction::callback(
        ctx,
        Some("op_read_readable_stream"),
        Some(op_read_readable_stream),
    );

    let op_read_sync_readable_stream_fn = JSFunction::callback(
        ctx,
        Some("op_read_sync_readable_stream"),
        Some(op_read_sync_readable_stream),
    );

    let op_write_readable_stream_fn = JSFunction::callback(
        ctx,
        Some("op_write_readable_stream"),
        Some(op_write_readable_stream),
    );

    let op_write_sync_readable_stream_fn = JSFunction::callback(
        ctx,
        Some("op_write_sync_readable_stream"),
        Some(op_write_sync_readable_stream),
    );

    let op_close_readable_stream_fn = JSFunction::callback(
        ctx,
        Some("op_close_readable_stream"),
        Some(op_close_readable_stream),
    );

    let op_wait_close_readable_stream_fn = JSFunction::callback(
        ctx,
        Some("op_wait_close_readable_stream"),
        Some(op_wait_close_readable_stream),
    );

    // Exports
    JSReadableStreamResource::template_object(ctx, exports).unwrap();

    exports
        .set_property(
            "op_read_readable_stream",
            &op_read_readable_stream_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property(
            "op_read_sync_readable_stream",
            &op_read_sync_readable_stream_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property(
            "op_write_readable_stream",
            &op_write_readable_stream_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property(
            "op_write_sync_readable_stream",
            &op_write_sync_readable_stream_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property(
            "op_close_readable_stream",
            &op_close_readable_stream_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property(
            "op_wait_close_readable_stream",
            &op_wait_close_readable_stream_fn,
            Default::default(),
        )
        .unwrap();
}

#[cfg(test)]
mod tests {
    use crate::tests::test_utils::new_runtime;

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

        assert_eq!(stream.read_async().await.unwrap(), Some(vec![0]));
        assert_eq!(stream.read_async().await.unwrap(), Some(vec![1]));
        assert_eq!(stream.read_async().await.unwrap(), Some(vec![2]));
        assert_eq!(stream.read_async().await.unwrap(), Some(vec![3]));
        assert_eq!(stream.read_async().await.unwrap(), Some(vec![4]));
    }

    #[tokio::test]
    async fn test_internal_stream_resource_async_close() {
        let mut stream = InternalStreamResource::<Vec<u8>>::new(5);
        for i in 0..5 {
            stream.write_async(vec![i as u8]).await.unwrap();
        }

        stream.close();
        let result = stream.read_async().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(vec![0]));
        let result = stream.write_async(vec![5]).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StreamError::Closed);
    }

    #[test]
    fn test_bounded_buffer_channel() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write(vec![i as u8]).unwrap();
        }

        assert_eq!(channel.read().unwrap(), vec![0]);
        assert_eq!(channel.read().unwrap(), vec![1]);
        assert_eq!(channel.read().unwrap(), vec![2]);
        assert_eq!(channel.read().unwrap(), vec![3]);
        assert_eq!(channel.read().unwrap(), vec![4]);
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
    async fn test_bounded_buffer_channel_async() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write_async(vec![i as u8]).await.unwrap();
        }

        assert_eq!(channel.read_async().await.unwrap(), Some(vec![0]));
        assert_eq!(channel.read_async().await.unwrap(), Some(vec![1]));
        assert_eq!(channel.read_async().await.unwrap(), Some(vec![2]));
        assert_eq!(channel.read_async().await.unwrap(), Some(vec![3]));
        assert_eq!(channel.read_async().await.unwrap(), Some(vec![4]));
    }

    #[test]
    fn test_bounded_buffer_channel_reader() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write(vec![i as u8]).unwrap();
        }

        let mut reader = channel.new_reader().unwrap();
        assert_eq!(reader.read().unwrap(), vec![0]);
        assert_eq!(reader.read().unwrap(), vec![1]);
        assert_eq!(reader.read().unwrap(), vec![2]);
        assert_eq!(reader.read().unwrap(), vec![3]);
        assert_eq!(reader.read().unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_bounded_buffer_channel_reader_async() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write_async(vec![i as u8]).await.unwrap();
        }

        let mut reader = channel.new_reader().unwrap();
        assert_eq!(reader.read_async().await.unwrap(), Some(vec![0]));
        assert_eq!(reader.read_async().await.unwrap(), Some(vec![1]));
        assert_eq!(reader.read_async().await.unwrap(), Some(vec![2]));
        assert_eq!(reader.read_async().await.unwrap(), Some(vec![3]));
        assert_eq!(reader.read_async().await.unwrap(), Some(vec![4]));
    }

    #[tokio::test]
    async fn test_bound_buffer_channel_limit() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write(vec![i as u8]).unwrap();
        }

        let future = async {
            channel.write_async(vec![5]).await.unwrap();
        };

        let result =
            tokio::time::timeout(std::time::Duration::from_secs(1), future).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_bound_buffer_channel_limit() {
        let channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in 0..5 {
            channel.write(vec![i as u8]).unwrap();
        }

        let result = channel.write(vec![5]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StreamError::ChannelFull);
    }

    // Readable Stream Resource tests
    #[tokio::test]
    async fn test_readable_stream_resource() {
        let mut rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import {
                ReadableStreamResource,
                op_read_sync_readable_stream,
                op_write_sync_readable_stream
            } from '@kedo/internal/utils';
            import { ReadableStream } from "@kedo/stream";
            import assert from "@kedo/assert";

            const autoAllocateStream = new ReadableStream({
              start(controller) {
                this.counter = 0;
              },
              pull(controller) {
                if (this.counter < 3) {
                  const chunk = new Uint8Array(1);
                  chunk[0] = this.counter;
                  controller.byobRequest.respondWithNewView(chunk);
                  this.counter++;
                } else {
                  controller.close();
                }
              },
              type: "bytes",
              autoAllocateChunkSize: 1,
            });

            const resource = new ReadableStreamResource(100);
            const reader = autoAllocateStream.getReader();
            for (let i = 0; i < 3; i++) {
                const { value, done } = await reader.read();
                if (done) {
                    break;
                }
                op_write_sync_readable_stream(resource, value);
            }

            let result = op_read_sync_readable_stream(resource);
            assert.ok(result[0] === 0, `Expected 0, got ${result}`);
            result = op_read_sync_readable_stream(resource);
            assert.ok(result[0] === 1, `Expected 1, got ${result}`);
            result = op_read_sync_readable_stream(resource);
            assert.ok(result[0] === 2, `Expected 2, got ${result}`);
            result = op_read_sync_readable_stream(resource);
            assert.ok(result === undefined, `Expected undefined, got ${result}`);
            "#,
            "index.js",
            None,
        );

        if let Err(e) = result {
            panic!("{}", e.message().unwrap());
        }

        rt.idle().await;
        assert!(result.is_ok());
    }
}
