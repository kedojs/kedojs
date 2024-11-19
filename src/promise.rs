use rust_jsc::{
    callback, class::ClassError, finalize, JSClass, JSClassAttribute, JSContext, JSError,
    JSFunction, JSObject, JSResult, JSValue, PrivateData,
};
use tokio::sync::oneshot;

use crate::{
    class_table::ClassTable,
    context::downcast_state,
    job::AsyncJobQueue,
    utils::{downcast_ref, drop_ptr},
};

#[derive(Debug, PartialEq)]
pub enum InternalPromiseStatus {
    FulFilled(JSValue),
    Rejected(JSValue),
    Error,
    Elapsed,
}

pub struct InternalPromiseNotifier {
    notifier: Option<oneshot::Sender<InternalPromiseStatus>>,
}

impl InternalPromiseNotifier {
    pub fn from_sender(notifier: oneshot::Sender<InternalPromiseStatus>) -> Self {
        Self {
            notifier: Some(notifier),
        }
    }

    pub fn resolve(&mut self, value: JSValue) {
        if let Some(notifier) = self.notifier.take() {
            let _ = notifier.send(InternalPromiseStatus::FulFilled(value));
        }
    }

    pub fn reject(&mut self, value: JSValue) {
        if let Some(notifier) = self.notifier.take() {
            let _ = notifier.send(InternalPromiseStatus::Rejected(value));
        }
    }
}

pub struct InternalPromiseSubscriber {
    receiver: oneshot::Receiver<InternalPromiseStatus>,
}

impl InternalPromiseSubscriber {
    pub fn new(receiver: oneshot::Receiver<InternalPromiseStatus>) -> Self {
        Self { receiver }
    }

    pub async fn wait(
        &mut self,
        timeout: Option<u16>,
    ) -> Result<InternalPromiseStatus, InternalPromiseStatus> {
        if let Some(timeout) = timeout {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(timeout.into()),
                &mut self.receiver,
            )
            .await;

            match result {
                Ok(Ok(value)) => Ok(value),
                Ok(Err(_)) => Err(InternalPromiseStatus::Error),
                Err(_) => Err(InternalPromiseStatus::Elapsed),
            }
        } else {
            tokio::select! {
                result = &mut self.receiver => {
                    match result {
                        Ok(value) => Ok(value),
                        Err(_) => Err(InternalPromiseStatus::Error),
                    }
                }
            }
        }
    }

    pub fn into_receiver(self) -> oneshot::Receiver<InternalPromiseStatus> {
        self.receiver
    }

    pub fn is_resolved(&mut self) -> bool {
        self.receiver.try_recv().is_ok()
    }
}

pub struct InternalPromise {}

impl InternalPromise {
    pub const CLASS_NAME: &'static str = "InternalPrmise";
    // pub const PROTO_NAME: &'static str = "InternalPrmisePrototype";

    pub fn init_class(manaager: &mut ClassTable) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            // .call_as_constructor(Some(Self::constructor))
            .set_finalize(Some(Self::finalize))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        manaager.insert(class);
        Ok(())
    }

    fn set_callbacks(ctx: &JSContext, this: JSObject, promise: JSObject) -> JSResult<()> {
        let resolved = JSFunction::callback(
            &ctx,
            Some("resolveCallback"),
            Some(Self::resolve_callback),
        );
        let resolved_bind = resolved
            .as_object()?
            .get_property("bind")?
            .as_object()?
            .call(Some(&resolved.into()), &[this.clone().into()])?;

        let rejected = JSFunction::callback(
            &ctx,
            Some("rejectCallback"),
            Some(Self::reject_callback),
        );
        let rejected_bind = rejected
            .as_object()?
            .get_property("bind")?
            .as_object()?
            .call(Some(&rejected.into()), &[this.clone().into()])?;

        promise
            .get_property("then")?
            .as_object()?
            .call(Some(&promise), &[resolved_bind, rejected_bind])?;
        Ok(())
    }

    /// finalize is called when the object is being garbage collected.
    /// This is the place to clean up any resources that the object may hold.
    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<InternalPromiseNotifier>(data_ptr);
    }

    pub fn from_promise(
        ctx: &JSContext,
        promise: JSObject,
    ) -> JSResult<InternalPromiseSubscriber> {
        let state = downcast_state::<AsyncJobQueue>(&ctx);
        let class = state.classes().get(Self::CLASS_NAME).unwrap();
        let (tx, rx) = oneshot::channel();
        let notifier = InternalPromiseNotifier::from_sender(tx);
        let object =
            class.object::<InternalPromiseNotifier>(ctx, Some(Box::new(notifier)));
        InternalPromise::set_callbacks(ctx, object.clone(), promise)?;
        Ok(InternalPromiseSubscriber::new(rx))
    }

    // #[constructor]
    // fn constructor(
    //     ctx: JSContext,
    //     constructor: JSObject,
    //     args: &[JSValue],
    // ) -> JSResult<JSValue> {
    //     let promise = args
    //         .get(0)
    //         .ok_or_else(|| JSError::new_typ(&ctx, "Missing Promise").unwrap())?
    //         .as_object()?;

    //     let state = downcast_state::<AsyncJobQueue>(&ctx);
    //     let class = state.classes().get(InternalPromise::CLASS_NAME).unwrap();
    //     let internal_promise = InternalPromise::new();
    //     let object =
    //         class.object::<InternalPromise>(&ctx, Some(Box::new(internal_promise)));

    //     InternalPromise::set_callbacks(&ctx, object.clone(), promise)?;
    //     object.set_prototype(&constructor);
    //     Ok(object.into())
    // }

    #[callback]
    fn reject_callback(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut internal_promise = downcast_ref::<InternalPromiseNotifier>(&this)
            .ok_or_else(|| JSError::new_typ(&ctx, "Invalid this object").unwrap())?;

        let value: JSValue = args
            .get(0)
            .and_then(|value| Some(value.clone()))
            .unwrap_or_else(|| JSValue::undefined(&ctx));

        internal_promise.reject(value);
        Ok(JSValue::undefined(&ctx))
    }

    #[callback]
    fn resolve_callback(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut notifier = downcast_ref::<InternalPromiseNotifier>(&this)
            .ok_or_else(|| JSError::new_typ(&ctx, "Invalid this object").unwrap())?;

        let value: JSValue = args
            .get(0)
            .and_then(|value| Some(value.clone()))
            .unwrap_or_else(|| JSValue::undefined(&ctx));

        notifier.resolve(value);
        Ok(JSValue::undefined(&ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_utils::new_runtime;

    #[tokio::test]
    async fn test_internal_promise() {
        let mut rt = new_runtime();
        let _ = rt.evaluate_module_from_source(
            r#"
            const promise = new Promise((resolve, reject) => {
                setTimeout(() => {
                    resolve('done');
                }, 1000);
            });
            globalThis.promise = promise;
        "#,
            "index.js",
            None,
        );

        let result = rt.evaluate_script("globalThis.promise", None);

        let promise = result.unwrap().as_object().unwrap();
        let internal_promise = InternalPromise::from_promise(&rt.context(), promise);
        if internal_promise.is_err() {
            panic!(
                "Expected InternalPromise {}",
                internal_promise.err().unwrap().message().unwrap()
            );
        }

        let mut subcriber = internal_promise.unwrap();
        let (_, value) = tokio::join!(rt.idle(), subcriber.wait(Some(2)));
        let value = value.unwrap();
        let string = match value {
            InternalPromiseStatus::FulFilled(js_value) => {
                js_value.as_string().unwrap().to_string()
            }
            _ => panic!("Expected value"),
        };
        assert_eq!(string, "done");
    }

    #[tokio::test]
    async fn test_internal_promise_reject() {
        let mut rt = new_runtime();
        let _ = rt.evaluate_module_from_source(
            r#"
            const promise = new Promise((resolve, reject) => {
                setTimeout(() => {
                    reject('error');
                }, 1000);
            });
            globalThis.promise = promise;
        "#,
            "index.js",
            None,
        );

        let result = rt.evaluate_script("globalThis.promise", None);

        let promise = result.unwrap().as_object().unwrap();
        let internal_promise = InternalPromise::from_promise(&rt.context(), promise);
        if internal_promise.is_err() {
            panic!(
                "Expected InternalPromise {}",
                internal_promise.err().unwrap().message().unwrap()
            );
        }

        let mut subcriber = internal_promise.unwrap();
        let (_, value) = tokio::join!(rt.idle(), subcriber.wait(Some(2)));
        let value = value.unwrap();
        let string = match value {
            InternalPromiseStatus::Rejected(js_value) => {
                js_value.as_string().unwrap().to_string()
            }
            _ => panic!("Expected value"),
        };
        assert_eq!(string, "error");
    }

    #[tokio::test]
    async fn test_internal_promise_timeout() {
        let mut rt = new_runtime();
        let _ = rt.evaluate_module_from_source(
            r#"
            const promise = new Promise((resolve, reject) => {
                setTimeout(() => {
                    resolve('done');
                }, 2000);
            });
            globalThis.promise = promise;
        "#,
            "index.js",
            None,
        );

        let result = rt.evaluate_script("globalThis.promise", None);

        let promise = result.unwrap().as_object().unwrap();
        let internal_promise = InternalPromise::from_promise(&rt.context(), promise);
        if internal_promise.is_err() {
            panic!(
                "Expected InternalPromise {}",
                internal_promise.err().unwrap().message().unwrap()
            );
        }

        let mut subcriber = internal_promise.unwrap();
        let (_, value) = tokio::join!(rt.idle(), subcriber.wait(Some(1)));
        assert!(value.is_err());
        let value = value.err().unwrap();
        assert_eq!(value, InternalPromiseStatus::Elapsed);
    }
}
