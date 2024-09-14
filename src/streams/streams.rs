use rust_jsc::class::ClassError;
use rust_jsc::{
    callback, constructor, finalize, JSClass, JSClassAttribute, JSContext, JSError,
    JSFunction, JSObject, JSPromise, JSResult, JSTypedArray, JSValue, PrivateData,
    PropertyDescriptorBuilder,
};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;

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
    sender: Sender<T>,
    receiver: Receiver<T>,
    close_notifier: Option<oneshot::Sender<()>>,
    close_receiver: oneshot::Receiver<()>,
    is_closed: Arc<AtomicBool>,
}

impl<T: Clone + Send + 'static> BoundedBufferChannel<T> {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = channel(capacity);

        let (close_notifier, close_receiver) = oneshot::channel();
        let is_closed = Arc::new(AtomicBool::new(false));
        Self {
            sender,
            receiver,
            close_notifier: Some(close_notifier),
            close_receiver,
            is_closed,
        }
    }

    pub fn write(&self, item: T) -> Result<(), StreamError> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(StreamError::Closed);
        }

        self.sender.try_send(item).map_err(|e| match e {
            tokio::sync::mpsc::error::TrySendError::Full(_) => StreamError::ChannelFull,
            _ => StreamError::SendError(e.to_string()),
        })
    }

    pub async fn write_async(&mut self, item: T) -> Result<(), StreamError> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(StreamError::Closed);
        }

        tokio::select! {
            biased;
            _ = &mut self.close_receiver => {
                self.close_receiver.close();
                Err(StreamError::Closed)
            },
            _ = self.sender.send(item) => Ok(()),
        }
    }

    pub fn read(&mut self) -> Result<T, StreamError> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(StreamError::Closed);
        }

        self.receiver.try_recv().map_err(|e| match e {
            tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
            tokio::sync::mpsc::error::TryRecvError::Disconnected => StreamError::Closed,
            _ => StreamError::SendError(e.to_string()),
        })
    }

    pub async fn read_async(&mut self) -> Result<Option<T>, StreamError> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(StreamError::Closed);
        }

        tokio::select! {
            biased;
            _ = &mut self.close_receiver => {
                self.close_receiver.close();
                Err(StreamError::Closed)
            },
            msg = self.receiver.recv() => Ok(msg),
        }
    }

    pub fn close(&mut self) {
        self.is_closed.store(true, Ordering::Relaxed);
        self.receiver.close();
        self.close_notifier.take().and_then(|tx| tx.send(()).ok());
    }
}

pub struct ReadableStreamResource<T> {
    channel: BoundedBufferChannel<T>,
}

impl<T: Clone + Send + 'static> ReadableStreamResource<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            channel: BoundedBufferChannel::new(capacity),
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

    pub fn close(&mut self) {
        self.channel.close()
    }

    pub fn is_closed(&self) -> bool {
        self.channel.is_closed.load(Ordering::Relaxed)
    }
}

pub struct JSReadableStreamResource {
    inner: JSObject,
    pull_algorithm: Option<JSFunction>,
}

impl JSReadableStreamResource {
    pub const CLASS_NAME: &'static str = "JSReadableStreamResource";
    pub const PROTO_NAME: &'static str = "JSReadableStreamResourcePrototype";

    pub fn from_object(ctx: &JSContext, object: JSObject) -> JSResult<Self> {
        let pull_algorithm = object.get_property("pullAlgorithm")?.as_object()?;
        if !pull_algorithm.is_function() {
            return Err(JSError::new_typ(ctx, "pullAlgorithm must be a function")?);
        }

        let pull_algorithm = JSFunction::from(pull_algorithm);
        Ok(Self {
            inner: object,
            pull_algorithm: Some(pull_algorithm),
        })
    }

    pub fn pull_next(&self) -> JSResult<()> {
        // let stream_resource =
        //     downcast_ref::<ReadableStreamResource<Vec<u8>>>(&self.inner).unwrap();
        if let Some(pull_algorithm) = &self.pull_algorithm {
            let result = pull_algorithm.call(Some(&self.inner), &[]);
            if result.is_err() {
                return Err(result.unwrap_err());
            }
        }

        Ok(())
    }

    pub fn init_proto(
        proto_manager: &mut ProtoTable,
        manager: &mut ClassTable,
        ctx: &JSContext,
    ) -> Result<(), ClassError> {
        let class = manager.get(JSReadableStreamResource::CLASS_NAME).unwrap();
        let template_object = class.object::<ReadableStreamResource<Vec<u8>>>(ctx, None);
        JSReadableStreamResource::set_properties(ctx, &template_object).unwrap();
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
        drop_ptr::<ReadableStreamResource<Vec<u8>>>(data_ptr);
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
        let pull_algorithm = args
            .get(1)
            .ok_or_else(|| {
                JSError::new_typ(&ctx, "Missing pull algorithm argument").unwrap()
            })?
            .as_object()?;

        let state = downcast_state::<AsyncJobQueue>(&ctx);
        let class = state
            .classes()
            .get(JSReadableStreamResource::CLASS_NAME)
            .unwrap();

        let stream_resource = ReadableStreamResource::new(high_water_mark as usize);
        let object = class.object::<ReadableStreamResource<Vec<u8>>>(
            &ctx,
            Some(Box::new(stream_resource)),
        );

        object.set_property("pullAlgorithm", &pull_algorithm, Default::default())?;
        object.set_prototype(&constructor);
        Ok(object.into())
    }

    #[callback]
    fn read(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = downcast_state::<AsyncJobQueue>(&ctx);
        let resource = downcast_ref::<ReadableStreamResource<Vec<u8>>>(&this);
        let mut readable_stream = if let Some(stream) = resource {
            stream
        } else {
            return Err(JSError::new_typ(&ctx, "Invalid this object").unwrap());
        };

        let (promise, resolver) = JSPromise::new_pending(&ctx)?;
        let future = async move {
            let result = readable_stream.read_async().await;
            NativeJob::new(move |ctx| {
                match result {
                    Ok(bytes) => {
                        let chunk = if let Some(mut bytes) = bytes {
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
            })
        };

        state.job_queue().borrow().spawn(Box::pin(future));
        Ok(promise.into())
    }

    #[callback]
    fn read_sync(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let resource = downcast_ref::<ReadableStreamResource<Vec<u8>>>(&this);
        let mut readable_stream = if let Some(stream) = resource {
            stream
        } else {
            return Err(JSError::new_typ(&ctx, "Invalid this object").unwrap());
        };

        match readable_stream.read() {
            Ok(mut bytes) => {
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
    fn write_sync(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let resource = downcast_ref::<ReadableStreamResource<Vec<u8>>>(&this);
        let readable_stream = if let Some(stream) = resource {
            stream
        } else {
            return Err(JSError::new_typ(&ctx, "Invalid this object").unwrap());
        };

        let chunk = args
            .get(0)
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
    fn write(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = downcast_state::<AsyncJobQueue>(&ctx);
        let resource = downcast_ref::<ReadableStreamResource<Vec<u8>>>(&this);
        let mut readable_stream = if let Some(stream) = resource {
            stream
        } else {
            return Err(JSError::new_typ(&ctx, "Invalid this object").unwrap());
        };

        let chunk = args
            .get(0)
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
                    Err(err) => {
                        let err_value =
                            JSError::with_message(ctx, format!("{}", err)).unwrap();
                        resolver.reject(Some(&this), &[err_value.into()])?;
                    }
                }
                Ok(())
            })
        };

        state.job_queue().borrow().spawn(Box::pin(future));
        Ok(promise.into())
    }

    #[callback]
    fn close(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let resource = downcast_ref::<ReadableStreamResource<Vec<u8>>>(&this);
        let mut readable_stream = if let Some(stream) = resource {
            stream
        } else {
            return Err(JSError::new_typ(&ctx, "Invalid this object").unwrap());
        };

        readable_stream.close();
        Ok(JSValue::undefined(&ctx))
    }

    fn set_properties(ctx: &JSContext, this: &JSObject) -> JSResult<()> {
        let descriptor = PropertyDescriptorBuilder::new()
            .writable(false)
            .enumerable(false)
            .configurable(false)
            .build();

        let function = JSFunction::callback(&ctx, Some("writeChunk"), Some(Self::write));
        this.set_property("writeChunk", &function, descriptor)?;

        let function =
            JSFunction::callback(&ctx, Some("writeChunkSync"), Some(Self::write_sync));
        this.set_property("writeChunkSync", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("readChunk"), Some(Self::read));
        this.set_property("readChunk", &function, descriptor)?;

        let function =
            JSFunction::callback(&ctx, Some("readChunkSync"), Some(Self::read_sync));
        this.set_property("readChunkSync", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("close"), Some(Self::close));
        this.set_property("close", &function, descriptor)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
