use rust_jsc::{
    callback, class::ClassError, constructor, finalize, has_instance, initialize,
    JSArray, JSClass, JSClassAttribute, JSContext, JSFunction, JSObject, JSResult,
    JSString, JSValue, JSValueType, PrivateData, PropertyDescriptorBuilder,
};

use crate::{
    class_table::ClassTable,
    context::{downcast_state, KedoContext},
    iterator::{JsIterator, JsIteratorResult, JsIteratorState, PropertyNameKind},
    job::AsyncJobQueue,
    proto_table::ProtoTable,
    utils::{downcast_ptr, downcast_ref, drop_ptr, upcast},
};

#[derive(Debug, Clone, Default)]
pub struct HeadersMap {
    // TOOD: consider using a HashMap here
    inner: Vec<(JSString, JSString)>,
}

impl HeadersMap {
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

// impl Drop for HeadersMap {
//     fn drop(&mut self) {
//         // println!("Dropping HeadersMap");
//     }
// }

impl From<JSObject> for HeadersMap {
    fn from(object: JSObject) -> Self {
        let mut headers = Vec::new();
        let properties_names = object.get_property_names();

        for key in properties_names {
            let value = object
                .get_property(key.to_string())
                .unwrap()
                .as_string()
                .unwrap();

            headers.push((key, value));
        }

        Self { inner: headers }
    }
}

impl From<JSArray> for HeadersMap {
    fn from(array: JSArray) -> Self {
        let length = array.length().unwrap() as u32;
        let mut headers = Vec::new();

        for i in 0..length {
            let entry: JSObject = array.get(i).unwrap().as_object().unwrap();
            let key = entry.get_property_at_index(0).unwrap().as_string().unwrap();
            let value = entry.get_property_at_index(1).unwrap().as_string().unwrap();
            headers.push((key, value));
        }

        Self { inner: headers }
    }
}

impl From<&JSValue> for HeadersMap {
    fn from(value: &JSValue) -> Self {
        match value.get_type() {
            JSValueType::Object => {
                let object = value.as_object().unwrap();
                if object.is_array() {
                    Self::from(JSArray::new(object))
                } else {
                    Self::from(object)
                }
            }
            _ => Self::default(),
        }
    }
}

pub struct HeadersIterator {}

impl HeadersIterator {
    pub const PROTO_NAME: &'static str = "HeadersIteratorPrototype";

    pub fn init_proto(
        proto_manager: &mut ProtoTable,
        manaager: &mut ClassTable,
        ctx: &JSContext,
    ) {
        let next = JSFunction::callback(ctx, Some("next"), Some(Self::next));
        let class = manaager.get(JsIterator::CLASS_NAME).unwrap();
        let object = class.object::<JsIteratorState>(ctx, None);

        object
            .set_property("next", &next, Default::default())
            .unwrap();

        let iterator_fn =
            JSFunction::callback::<JSString>(&ctx, None, Some(JsIterator::iterator));
        object
            .set_iterator(&iterator_fn.into(), Default::default())
            .unwrap();
        proto_manager.insert(Self::PROTO_NAME.to_string(), object);
    }

    pub fn new(
        ctx: &JSContext,
        headers: Box<HeadersMap>,
        kind: PropertyNameKind,
    ) -> JSResult<JSObject> {
        let state = JsIteratorState {
            data: upcast::<HeadersMap>(headers),
            index: 0,
            kind,
        };

        let object = JsIterator::new(ctx, state, None)?;
        let runtime_state = downcast_state::<AsyncJobQueue>(ctx);
        let proto = runtime_state.protos().get(Self::PROTO_NAME).unwrap();
        object.set_prototype(proto);
        Ok(object)
    }

    #[callback]
    fn next(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut state = downcast_ref::<JsIteratorState>(&this).unwrap();
        let headers = downcast_ptr::<HeadersMap>(state.data).unwrap();
        let index = state.index;
        let kind = state.kind;

        if index >= headers.len() {
            let value = JSValue::undefined(&ctx);
            let object = JsIteratorResult::new_object(&ctx, true, &value)?;
            return Ok(object.into());
        }

        let (key, value) = &headers.inner[index];

        match kind {
            PropertyNameKind::Key => {
                let result = JsIteratorResult::new_object(
                    &ctx,
                    false,
                    &JSValue::string(&ctx, key.to_string()),
                )?;
                state.index += 1;
                return Ok(result.into());
            }
            PropertyNameKind::Value => {
                let result = JsIteratorResult::new_object(
                    &ctx,
                    false,
                    &JSValue::string(&ctx, value.to_string()),
                )?;
                state.index += 1;
                return Ok(result.into());
            }
            PropertyNameKind::KeyAndValue => {
                let array = JSArray::new_array(
                    &ctx,
                    &[
                        JSValue::string(&ctx, key.to_string()),
                        JSValue::string(&ctx, value.to_string()),
                    ],
                )?;

                let value = array.into();
                let result = JsIteratorResult::new_object(&ctx, false, &value)?;
                state.index += 1;
                return Ok(result.into());
            }
        }
    }
}

pub struct Headers {}

impl Headers {
    pub const CLASS_NAME: &'static str = "Headers";

    pub fn init(
        manaager: &mut ClassTable,
        ctx: &JSContext,
        global: &JSObject,
    ) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .call_as_constructor(Some(Self::constructor))
            .set_finalize(Some(Self::finalize))
            .has_instance(Some(Self::has_instance))
            .set_initialize(Some(Self::initialize))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        let template_object = class.object::<HeadersMap>(ctx, None);
        Headers::set_properties(&ctx, &template_object).unwrap();
        global
            .set_property(Headers::CLASS_NAME, &template_object, Default::default())
            .unwrap();

        manaager.insert(class);
        Ok(())
    }

    pub fn is(ctx: &KedoContext, object: &JSObject) -> JSResult<bool> {
        let state = ctx.state();
        let header_class = state.classes().get(Headers::CLASS_NAME).unwrap();
        object.is_object_of_class(header_class)
    }

    fn set_properties(ctx: &JSContext, this: &JSObject) -> JSResult<()> {
        let descriptor = PropertyDescriptorBuilder::new()
            .writable(false)
            .enumerable(false)
            .configurable(false)
            .build();

        let function = JSFunction::callback(&ctx, Some("append"), Some(Self::append));
        this.set_property("append", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("delete"), Some(Self::delete));
        this.set_property("delete", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("get"), Some(Self::get));
        this.set_property("get", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("set"), Some(Self::set));
        this.set_property("set", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("has"), Some(Self::has));
        this.set_property("has", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("keys"), Some(Self::keys));
        this.set_property("keys", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("values"), Some(Self::values));
        this.set_property("values", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("entries"), Some(Self::iterator));
        this.set_property("entries", &function, descriptor)?;

        let function = JSFunction::callback::<JSString>(&ctx, None, Some(Self::iterator));
        this.set_iterator(&function.into(), descriptor)?;

        Ok(())
    }

    // pub fn new_object(ctx: &KedoContext, headers: HeadersMap) -> JSResult<JSObject> {
    //     let state = ctx.state();
    //     let class = state.classes().get(Headers::CLASS_NAME).unwrap();
    //     let object = class.object::<HeadersMap>(&ctx, Some(Box::new(headers)));
    //     Headers::set_properties(&ctx, &object)?;
    //     Ok(object)
    // }

    /// Create a new Headers object and set the prototype to the Headers Prototype.
    #[constructor]
    fn constructor(
        ctx: JSContext,
        constructor: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let context = KedoContext::from(&ctx);
        let binding = context.state();
        let header_class = binding.classes().get(Headers::CLASS_NAME).unwrap();
        let headers = if args.is_empty() {
            HeadersMap::default()
        } else {
            HeadersMap::from(&args[0])
        };

        let object = header_class.object::<HeadersMap>(&ctx, Some(Box::new(headers)));
        object.set_prototype(&constructor);
        Ok(object.into())
    }

    #[initialize]
    fn initialize(_ctx: JSContext, _this: JSObject) {}

    /// finalize is called when the object is being garbage collected.
    /// This is the place to clean up any resources that the object may have.
    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<HeadersMap>(data_ptr);
    }

    /// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Symbol/hasInstance
    ///
    /// The Symbol.hasInstance well-known symbol is used to determine if a constructor object recognizes an object as its instance.
    /// The instanceof operator's behavior can be customized by this symbol.
    ///
    /// Check if the object is an instance of Headers class.
    #[has_instance]
    fn has_instance(
        ctx: JSContext,
        _constructor: JSObject,
        value: JSValue,
    ) -> JSResult<bool> {
        let context = KedoContext::from(&ctx);
        let object = value.as_object()?;
        Headers::is(&context, &object)
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/entries
    ///
    /// The Headers.entries() method returns an iterator allowing to go through
    /// all key/value pairs contained in this object.
    /// Both the key and value of each pair are String objects.
    ///
    /// Syntax
    ///    myHeaders.entries();
    #[callback]
    fn iterator(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let iterator =
            HeadersIterator::new(&ctx, headers.take(), PropertyNameKind::KeyAndValue)?;
        Ok(iterator.into())
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/keys
    ///
    /// The Headers.keys() method returns an iterator allowing to go through all keys contained in this object.
    /// The keys are String objects.
    ///
    /// Syntax
    ///    myHeaders.keys();
    #[callback]
    fn keys(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let headers = downcast_ref::<HeadersMap>(&this).unwrap();
        // let mut headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let object = HeadersIterator::new(&ctx, headers.take(), PropertyNameKind::Key)?;
        Ok(object.into())
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/values
    ///
    /// The Headers.values() method returns an iterator allowing to go through
    /// all values contained in this object. The values are String objects.
    ///
    /// Syntax
    ///   myHeaders.values();
    #[callback]
    fn values(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let headers = downcast_ref::<HeadersMap>(&this).unwrap();
        // let mut headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let iterator =
            HeadersIterator::new(&ctx, headers.take(), PropertyNameKind::Value)?;
        Ok(iterator.into())
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/append
    ///
    /// Syntax
    ///     myHeaders.append(name, value);
    /// The append() method of the Headers interface appends a new value onto an
    /// existing header inside a Headers object, or adds the header if it does not already exist.
    /// The difference between set() and append() is that if the specified header already exists
    /// and accepts multiple values,
    /// set() will overwrite the existing value with the new one,
    /// whereas append() will append the new value onto the end of the set of values.
    #[callback]
    fn append(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let key = args[0].as_string()?;
        let value = args[1].as_string()?;
        // headers.inner.insert(key.to_string(), value.to_string());
        headers.inner.push((key, value));

        Ok(JSValue::undefined(&ctx))
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/has
    ///
    /// Syntax
    ///    myHeaders.has(name);
    ///
    /// The has() method of the Headers interface returns a boolean
    /// stating whether a Headers object contains a certain header.
    #[callback]
    fn has(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let key = args[0].as_string()?;
        // let has = headers.inner.contains_key(&key.to_string());
        let has = headers.inner.iter().any(|(k, _)| k == &key.to_string());

        Ok(JSValue::boolean(&ctx, has))
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/delete
    ///
    /// Syntax
    ///   myHeaders.delete(name);
    ///
    /// The delete() method of the Headers interface deletes a header from the current Headers object.
    #[callback]
    fn delete(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let key = args[0].as_string()?;
        // headers.inner.remove(&key.to_string());
        headers.inner.retain(|(k, _)| k != &key.to_string());

        Ok(JSValue::undefined(&ctx))
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/set
    ///
    /// Syntax
    ///  myHeaders.set(name, value);
    ///
    /// The set() method of the Headers interface sets a new value for an existing header
    /// inside a Headers object, or adds the header if it does not already exist.
    /// The difference between set() and Headers.append is that if the specified header
    /// already exists and accepts multiple values,
    /// set() overwrites the existing value with the new one, whereas Headers.append appends
    /// the new value to the end of the set of values.
    #[callback]
    fn set(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let key = args[0].as_string()?;
        let value = args[1].as_string()?;
        // headers.inner.insert(key.to_string(), value.to_string());
        if let Some(header) = headers
            .inner
            .iter_mut()
            .find(|(k, _)| k == &key.to_string())
        {
            header.1 = value;
        } else {
            headers.inner.push((key, value));
        }

        Ok(JSValue::undefined(&ctx))
    }

    /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/get
    ///
    /// Syntax
    ///    myHeaders.get(name);
    ///
    /// The get() method of the Headers interface returns a byte string of all
    /// the values of a header within a Headers object with a given name.
    /// If the requested header doesn't exist in the Headers object, it returns null.
    #[callback]
    fn get(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let headers = downcast_ref::<HeadersMap>(&this).unwrap();
        let key = args[0].as_string()?;
        // let value = headers.inner.get(&key.to_string()).map(|v| v.as_str());
        let value = headers
            .inner
            .iter()
            .find(|(k, _)| k == &key.to_string())
            .map(|(_, v)| v.to_string());

        if let Some(value) = value {
            return Ok(JSValue::string(&ctx, value));
        }

        Ok(JSValue::null(&ctx))
    }
}
