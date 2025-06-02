use futures::channel::oneshot;
use kedo_core::{downcast_state, ClassTable, ProtoTable};
use kedo_utils::{downcast_ref, drop_ptr};
use rust_jsc::{
    callback, class::ClassError, constructor, finalize, JSClass, JSClassAttribute,
    JSContext, JSError, JSFunction, JSObject, JSResult, JSValue, PrivateData,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct OneshotSignal {
    receiver: Option<oneshot::Receiver<()>>,
}

impl OneshotSignal {
    pub fn new() -> (Self, oneshot::Sender<()>) {
        let (sender, receiver) = oneshot::channel();
        (
            Self {
                receiver: Some(receiver),
            },
            sender,
        )
    }

    pub fn poll_signal(&mut self, cx: &mut Context) -> Poll<Result<(), ()>> {
        if let Some(receiver) = self.receiver.as_mut() {
            match Pin::new(receiver).poll(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(_)) => Poll::Ready(Err(())),
            }
        } else {
            Poll::Ready(Err(()))
        }
    }

    pub async fn wait(&mut self) -> Result<(), ()> {
        self.receiver.take().ok_or(())?.await.map_err(|_| ())
    }
}

#[derive(Debug)]
pub struct InternalSignal {
    sender: Option<oneshot::Sender<()>>,
    signal: Option<OneshotSignal>,
}

impl InternalSignal {
    pub const CLASS_NAME: &'static str = "InternalSignal";
    pub const PROTO_NAME: &'static str = "InternalSignalPrototype";

    pub fn get_signal(&mut self) -> Option<OneshotSignal> {
        self.signal.take()
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

    pub fn init_proto(
        proto_manager: &mut ProtoTable,
        manager: &mut ClassTable,
        ctx: &JSContext,
    ) -> Result<(), ClassError> {
        let class = manager.get(InternalSignal::CLASS_NAME).unwrap();
        let template_object = class.object::<InternalSignal>(ctx, None);
        proto_manager.insert(InternalSignal::PROTO_NAME.to_string(), template_object);
        Ok(())
    }

    pub fn template_object(ctx: &JSContext, scope: &JSObject) -> JSResult<()> {
        let state = downcast_state(ctx);
        let template_object = state
            .protos()
            .get(InternalSignal::PROTO_NAME)
            .expect("Expected InternalSignalPrototype to be defined");
        scope.set_property(
            InternalSignal::CLASS_NAME,
            &template_object,
            Default::default(),
        )?;
        Ok(())
    }

    /// finalize is called when the object is being garbage collected.
    /// This is the place to clean up any resources that the object may hold.
    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<InternalSignal>(data_ptr);
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        constructor: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = downcast_state(&ctx);
        let class = state.classes().get(InternalSignal::CLASS_NAME).unwrap();
        let (oneshot_signal, sender) = OneshotSignal::new();
        let internal_signal = InternalSignal {
            sender: Some(sender),
            signal: Some(oneshot_signal),
        };
        let object =
            class.object::<InternalSignal>(&ctx, Some(Box::new(internal_signal)));

        object.set_prototype(&constructor);
        Ok(object.into())
    }
}

#[callback]
fn op_send_signal(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let resource_args = match args.get(0) {
        Some(value) => value.as_object()?,
        None => return Err(JSError::new_typ(&ctx, "Missing arguments")?),
    };

    if let Some(mut resource) = downcast_ref::<InternalSignal>(&resource_args) {
        match resource.sender.take() {
            Some(sender) => {
                let _ = sender.send(());
            }
            _ => return Err(JSError::new_typ(&ctx, "Missing sender")?),
        };
    }

    Ok(JSValue::undefined(&ctx))
}

pub fn signal_exports(ctx: &JSContext, exports: &JSObject) {
    let op_send_signal_fn =
        JSFunction::callback(ctx, Some("op_send_signal"), Some(op_send_signal));

    InternalSignal::template_object(ctx, exports)
        .expect("Failed to set InternalSignal prototype");

    exports
        .set_property("op_send_signal", &op_send_signal_fn, Default::default())
        .expect("Failed to set op_send_signal");
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::noop_waker;
    use std::task::Context;

    #[test]
    fn test_signal() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let (mut signal, notifier) = OneshotSignal::new();
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        let notified = Arc::new(AtomicBool::new(false));
        let notified_clone = Arc::clone(&notified);
        let future = async move {
            signal.wait().await.unwrap();
            notified_clone.store(true, Ordering::SeqCst);
        };

        let mut future = Box::pin(future);
        assert_eq!(notified.load(Ordering::SeqCst), false);
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);

        notifier.send(()).unwrap();
        assert_eq!(notified.load(Ordering::SeqCst), false);
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(()));
        assert_eq!(notified.load(Ordering::SeqCst), true);
    }
}
