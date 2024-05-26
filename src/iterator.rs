use rust_jsc::{
    callback, class::ClassError, finalize, initialize, JSClass, JSClassAttribute,
    JSContext, JSFunction, JSObject, JSResult, JSValue, PrivateData, PropertyDescriptor,
};

use crate::{
    class_table::ClassTable, context::downcast_state, job::AsyncJobQueue, utils::drop_ptr,
};

#[derive(Debug, Clone, Copy)]
pub enum PropertyNameKind {
    Key,
    Value,
    KeyAndValue,
}

pub struct JsIteratorState {
    pub data: PrivateData,
    pub index: usize,
    pub kind: PropertyNameKind,
}

// impl Drop for JsIteratorState {
//     fn drop(&mut self) {
//         println!("Dropping JsIteratorState")
//     }
// }

pub struct JsIteratorResult {
    pub done: bool,
    pub value: JSValue,
}

impl JsIteratorResult {
    pub fn new_object(
        ctx: &JSContext,
        done: bool,
        value: &JSValue,
    ) -> JSResult<JSObject> {
        create_iterator_result(ctx, done, value)
    }
}

fn create_iterator_result(
    ctx: &JSContext,
    done: bool,
    value: &JSValue,
) -> JSResult<JSObject> {
    let object = JSObject::new(ctx);
    let done = JSValue::boolean(ctx, done);
    object.set_property("done", &done, Default::default())?;
    object.set_property("value", value, Default::default())?;
    Ok(object)
}

pub struct JsIterator {}

impl JsIterator {
    pub const CLASS_NAME: &'static str = "Iterator";

    pub fn init(manaager: &mut ClassTable) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .set_finalize(Some(Self::finalize))
            .set_initialize(Some(Self::initialize))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        manaager.insert(class);
        Ok(())
    }

    pub fn new(
        ctx: &JSContext,
        state: JsIteratorState,
        next: Option<&JSFunction>,
    ) -> JSResult<JSObject> {
        let binding = downcast_state::<AsyncJobQueue>(ctx);
        let class = binding.classes().get(Self::CLASS_NAME).unwrap();
        let object = class.object::<JsIteratorState>(ctx, Some(Box::new(state)));

        if let Some(next) = next {
            object.set_property("next", next, PropertyDescriptor::default())?;
        }

        Ok(object)
    }

    #[callback]
    pub fn iterator(
        _ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        Ok(this.into())
    }

    #[initialize]
    fn initialize(_ctx: JSContext, _this: JSObject) {}

    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<JsIteratorState>(data_ptr);
    }
}
