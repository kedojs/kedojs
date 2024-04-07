use std::collections::HashMap;

use boa_engine::{
  builtins::iterable::create_iter_result_object,
  class::Class,
  js_string,
  object::{builtins::JsArray, FunctionObjectBuilder},
  property::PropertyDescriptor,
  Context, JsData, JsNativeError, JsObject, JsResult, JsSymbol, JsValue, NativeFunction,
};
use boa_gc::{Finalize, Trace};
use hyper::{
  header::{HeaderName, HeaderValue},
  HeaderMap,
};
use serde_json::Value;

use crate::util::{js_function, PropertyNameKind};

pub trait WebHeaders {
  fn append(this: &JsValue, args: &[JsValue], context: &mut Context)
    -> JsResult<JsValue>;
  fn delete(this: &JsValue, args: &[JsValue], context: &mut Context)
    -> JsResult<JsValue>;
  fn get(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue>;
  fn set(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue>;
  fn has(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue>;
  fn entries(
    this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
  ) -> JsResult<JsValue>;
  fn keys(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue>;
  fn values(this: &JsValue, args: &[JsValue], context: &mut Context)
    -> JsResult<JsValue>;
}

#[derive(Debug, Clone, Default, Trace, Finalize, JsData)]
pub struct Headers {
  headers: HashMap<String, String>,
}

impl Into<HeaderMap<HeaderValue>> for Headers {
  fn into(self) -> HeaderMap<HeaderValue> {
    let mut headers = HeaderMap::new();
    for (key, value) in &self.headers {
      headers.insert(
        HeaderName::from_bytes(key.as_bytes()).unwrap(),
        HeaderValue::from_str(&value).unwrap(),
      );
    }

    headers
  }
}

impl From<HeaderMap<HeaderValue>> for Headers {
  fn from(headers: HeaderMap<HeaderValue>) -> Self {
    let mut map = HashMap::new();
    for (key, value) in headers.iter() {
      map.insert(
        key.as_str().to_string(),
        value.to_str().unwrap().to_string(),
      );
    }
    Headers { headers: map }
  }
}

impl From<HashMap<String, String>> for Headers {
  fn from(headers: HashMap<String, String>) -> Self {
    Headers { headers }
  }
}

impl From<Value> for Headers {
  fn from(value: Value) -> Self {
    let mut headers = Headers::from(HeaderMap::new());

    match value {
      Value::Null => {}
      Value::Bool(_) => {}
      Value::Number(_) => {}
      Value::String(_) => {}
      Value::Array(data) => {
        for value in data {
          let value = value.as_array().expect("value is not an array");
          let key = value.get(0).expect("no key provided").as_str().unwrap();
          let value = value.get(1).expect("no value provided").as_str().unwrap();
          headers.headers.insert(key.to_string(), value.to_string());
        }
      }
      Value::Object(data) => {
        for (key, value) in data {
          let value = value.as_str().expect("value is not a string");
          headers.headers.insert(key.to_string(), value.to_string());
        }
      }
    }

    headers
  }
}

impl Headers {
  fn to_string(
    this: &JsValue,
    _: &[JsValue],
    _context: &mut Context,
  ) -> JsResult<JsValue> {
    let headers = this
      .as_object()
      .and_then(JsObject::downcast_ref::<Self>)
      .ok_or_else(|| JsNativeError::typ().with_message("`this` is not a Headers"))?;

    Ok(JsValue::new(js_string!(format!("{:?}", headers.headers))))
  }

  pub fn to_object(&self, context: &mut Context) -> JsResult<JsValue> {
    let instance = JsObject::from_proto_and_data(None, self.clone());

    Self::define_object_property(&instance, context)?;
    Ok(instance.into())
  }

  fn define_object_property(instance: &JsObject, context: &mut Context) -> JsResult<()> {
    let append_fn = FunctionObjectBuilder::new(
      context.realm(),
      NativeFunction::from_fn_ptr(Self::append),
    )
    .name("append")
    .build();

    let delete_fn = FunctionObjectBuilder::new(
      context.realm(),
      NativeFunction::from_fn_ptr(Self::delete),
    )
    .name("delete")
    .build();

    let get_fn =
      FunctionObjectBuilder::new(context.realm(), NativeFunction::from_fn_ptr(Self::get))
        .name("get")
        .build();

    let set_fn =
      FunctionObjectBuilder::new(context.realm(), NativeFunction::from_fn_ptr(Self::set))
        .name("set")
        .build();

    let has_fn =
      FunctionObjectBuilder::new(context.realm(), NativeFunction::from_fn_ptr(Self::has))
        .name("has")
        .build();

    let entries_fn = FunctionObjectBuilder::new(
      context.realm(),
      NativeFunction::from_fn_ptr(Self::entries),
    )
    .name("entries")
    .build();

    let iterator_fn = FunctionObjectBuilder::new(
      context.realm(),
      NativeFunction::from_fn_ptr(Self::entries),
    )
    .name(JsSymbol::iterator().to_string())
    .build();

    let keys_fn = FunctionObjectBuilder::new(
      context.realm(),
      NativeFunction::from_fn_ptr(Self::keys),
    )
    .name("keys")
    .build();

    let values_fn = FunctionObjectBuilder::new(
      context.realm(),
      NativeFunction::from_fn_ptr(Self::values),
    )
    .name("values")
    .build();

    let to_string_fn = FunctionObjectBuilder::new(
      context.realm(),
      NativeFunction::from_fn_ptr(Headers::to_string),
    )
    .name("toString")
    .build();

    instance.define_property_or_throw(
      js_string!("append"),
      PropertyDescriptor::builder()
        .value(append_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("delete"),
      PropertyDescriptor::builder()
        .value(delete_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("get"),
      PropertyDescriptor::builder()
        .value(get_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("set"),
      PropertyDescriptor::builder()
        .value(set_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("has"),
      PropertyDescriptor::builder()
        .value(has_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("entries"),
      PropertyDescriptor::builder()
        .value(entries_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      JsSymbol::iterator(),
      PropertyDescriptor::builder()
        .value(iterator_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("keys"),
      PropertyDescriptor::builder()
        .value(keys_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("values"),
      PropertyDescriptor::builder()
        .value(values_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    instance.define_property_or_throw(
      js_string!("toString"),
      PropertyDescriptor::builder()
        .value(to_string_fn)
        .writable(false)
        .enumerable(false)
        .configurable(false),
      context,
    )?;

    Ok(())
  }
}

impl WebHeaders for Headers {
  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/append
  ///
  /// Syntax
  ///     myHeaders.append(name, value);
  /// The append() method of the Headers interface appends a new value onto an existing header inside a Headers object, or adds the header if it does not already exist.
  /// The difference between set() and append() is that if the specified header already exists and accepts multiple values,
  /// set() will overwrite the existing value with the new one, whereas append() will append the new value onto the end of the set of values.
  fn append(
    this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
  ) -> JsResult<JsValue> {
    let mut headers = this
      .as_object()
      .and_then(JsObject::downcast_mut::<Self>)
      .ok_or_else(|| JsNativeError::typ().with_message("`this` is not a Headers"))?;

    let key = args
      .get(0)
      .ok_or_else(|| JsNativeError::typ().with_message("No key argument provided"))?
      .as_string()
      .ok_or_else(|| JsNativeError::typ().with_message("Key argument is not a string"))?
      .to_std_string_escaped();
    let value = args
      .get(1)
      .ok_or_else(|| JsNativeError::typ().with_message("No value argument provided"))?
      .as_string()
      .ok_or_else(|| JsNativeError::typ().with_message("Value argument is not a string"))?
      .to_std_string_escaped();

    // insert or concatenate the value separated by a comma if the key already exists
    headers
      .headers
      .entry(key)
      .and_modify(|v| {
        *v = format!("{}, {}", v, value);
      })
      .or_insert(value);
    Ok(JsValue::undefined())
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/get
  ///
  /// Syntax
  ///    myHeaders.get(name);
  ///
  /// The get() method of the Headers interface returns a byte string of all the values of a header within a Headers object with a given name.
  /// If the requested header doesn't exist in the Headers object, it returns null.
  fn get(this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let headers = this
      .as_object()
      .and_then(JsObject::downcast_ref::<Self>)
      .ok_or_else(|| JsNativeError::typ().with_message("`this` is not a Headers"))?;

    let key = args
      .get(0)
      .ok_or_else(|| JsNativeError::typ().with_message("No name argument provided"))?
      .as_string()
      .ok_or_else(|| JsNativeError::typ().with_message("Name argument is not a string"))?
      .to_std_string_escaped();

    let value = headers.headers.get(&key).map(|v| v.as_str());

    if let Some(value) = value {
      return Ok(JsValue::new(js_string!(value)));
    }

    Ok(JsValue::null())
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/has
  ///
  /// Syntax
  ///    myHeaders.has(name);
  ///
  /// The has() method of the Headers interface returns a boolean stating whether a Headers object contains a certain header.
  fn has(this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let headers = this
      .as_object()
      .and_then(JsObject::downcast_ref::<Self>)
      .ok_or_else(|| JsNativeError::typ().with_message("`this` is not a Headers"))?;

    let key = args
      .get(0)
      .ok_or_else(|| JsNativeError::typ().with_message("No name argument provided"))?
      .as_string()
      .ok_or_else(|| JsNativeError::typ().with_message("Name argument is not a string"))?
      .to_std_string_escaped();

    let value = headers.headers.contains_key(&key);

    Ok(JsValue::new(value))
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/delete
  ///
  /// Syntax
  ///   myHeaders.delete(name);
  ///
  /// The delete() method of the Headers interface deletes a header from the current Headers object.
  fn delete(
    this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
  ) -> JsResult<JsValue> {
    let mut headers = this
      .as_object()
      .and_then(JsObject::downcast_mut::<Self>)
      .ok_or_else(|| JsNativeError::typ().with_message("`this` is not a Headers"))?;

    let key = args
      .get(0)
      .ok_or_else(|| JsNativeError::typ().with_message("No name argument provided"))?
      .as_string()
      .ok_or_else(|| JsNativeError::typ().with_message("Name argument is not a string"))?
      .to_std_string_escaped();

    headers.headers.remove(&key);
    Ok(JsValue::undefined())
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/set
  ///
  /// Syntax
  ///  myHeaders.set(name, value);
  ///
  /// The set() method of the Headers interface sets a new value for an existing header inside a Headers object, or adds the header if it does not already exist.
  /// The difference between set() and Headers.append is that if the specified header already exists and accepts multiple values,
  /// set() overwrites the existing value with the new one, whereas Headers.append appends the new value to the end of the set of values.
  fn set(this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let mut headers = this
      .as_object()
      .and_then(JsObject::downcast_mut::<Self>)
      .ok_or_else(|| JsNativeError::typ().with_message("`this` is not a Headers"))?;

    let key = args
      .get(0)
      .ok_or_else(|| JsNativeError::typ().with_message("No key argument provided"))?
      .as_string()
      .ok_or_else(|| JsNativeError::typ().with_message("Key argument is not a string"))?
      .to_std_string_escaped();
    let value = args
      .get(1)
      .ok_or_else(|| JsNativeError::typ().with_message("No value argument provided"))?
      .as_string()
      .ok_or_else(|| JsNativeError::typ().with_message("Value argument is not a string"))?
      .to_std_string_escaped();

    headers.headers.insert(key, value);

    Ok(JsValue::undefined())
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/entries
  ///
  /// Syntax
  ///  myHeaders.entries();
  ///
  /// The Headers.entries() method returns an iterator allowing to go through all key/value pairs contained in this object.
  /// Both the key and value of each pair are String objects.
  fn entries(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    HeadersIterator::create_headers_iterator(this, PropertyNameKind::KeyAndValue, context)
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/keys
  ///
  /// Syntax
  /// myHeaders.keys();
  ///
  /// The Headers.keys() method returns an iterator allowing to go through all keys contained in this object.
  /// The keys are String objects.
  fn keys(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    HeadersIterator::create_headers_iterator(this, PropertyNameKind::Key, context)
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/values
  /// Syntax
  /// myHeaders.values();
  ///
  /// The Headers.values() method returns an iterator allowing to go through all values contained in this object.
  /// The values are String objects.
  fn values(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    HeadersIterator::create_headers_iterator(this, PropertyNameKind::Value, context)
  }
}

#[derive(Debug, Finalize, Trace, JsData)]
pub struct HeadersIterator {
  headers: HashMap<String, String>,
  next_index: usize,
  #[unsafe_ignore_trace]
  kind: PropertyNameKind,
}

impl HeadersIterator {
  fn create_headers_iterator(
    headers: &JsValue,
    kind: PropertyNameKind,
    context: &mut Context,
  ) -> JsResult<JsValue> {
    let headers = headers
      .as_object()
      .and_then(JsObject::downcast_ref::<Headers>)
      .ok_or_else(|| JsNativeError::typ().with_message("`this` is not a Headers"))?;

    let iter = Self {
      headers: headers.headers.clone(),
      next_index: 0,
      kind,
    };
    let map_iterator = JsObject::from_proto_and_data(
      context
        .intrinsics()
        .objects()
        .iterator_prototypes()
        .iterator(),
      iter,
    );

    map_iterator.define_property_or_throw(
      JsSymbol::to_string_tag(),
      PropertyDescriptor::builder()
        .value(js_string!("Headers Iterator"))
        .writable(false)
        .enumerable(false)
        .configurable(true),
      context,
    )?;

    map_iterator.define_property_or_throw(
      js_string!("next"),
      PropertyDescriptor::builder()
        .value(js_function(
          context,
          NativeFunction::from_fn_ptr(Self::next),
          "next",
          0,
        ))
        .writable(true)
        .enumerable(false)
        .configurable(true),
      context,
    )?;

    Ok(map_iterator.into())
  }

  fn next(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // First, safely extract the necessary information without mutating anything
    let (key, iter_kind) = {
      // Temporarily create a mutable reference to headers_iterator for non-mutating operations
      let headers_iterator = this
        .as_object()
        .and_then(JsObject::downcast_ref::<Self>) // Use downcast_ref for immutable access
        .ok_or_else(|| {
          JsNativeError::typ().with_message("`this` is not a HeadersIterator")
        })?;

      // Clone key and kind to use them outside of this scope without holding a reference
      let key = headers_iterator
        .headers
        .keys()
        .nth(headers_iterator.next_index)
        .cloned();
      let iter_kind = headers_iterator.kind.clone(); // Assume cloning is trivial here

      (key, iter_kind)
    };

    let mut headers_iterator = this
      .as_object()
      .and_then(JsObject::downcast_mut::<Self>)
      .ok_or_else(|| {
        JsNativeError::typ().with_message("`this` is not a HeadersIterator")
      })?;

    if let Some(key) = key {
      headers_iterator.next_index += 1;

      let value = headers_iterator.headers.get(&key).unwrap();
      let result = match iter_kind {
        PropertyNameKind::Key => Ok(create_iter_result_object(
          JsValue::new(js_string!(key.clone())),
          false,
          context,
        )),
        PropertyNameKind::Value => Ok(create_iter_result_object(
          JsValue::new(js_string!(value.clone())),
          false,
          context,
        )),
        PropertyNameKind::KeyAndValue => {
          let result = JsArray::from_iter(
            [
              JsValue::new(js_string!(key.clone())),
              JsValue::new(js_string!(value.clone())),
            ],
            context,
          );
          Ok(create_iter_result_object(result.into(), false, context))
        }
      };

      return result;
    }

    Ok(create_iter_result_object(
      JsValue::undefined(),
      true,
      context,
    ))
  }
}

impl Class for Headers {
  const NAME: &'static str = "Headers";

  fn object_constructor(
    instance: &JsObject,
    _args: &[JsValue],
    context: &mut Context,
  ) -> JsResult<()> {
    Self::define_object_property(instance, context)?;

    Ok(())
  }

  fn init(_class: &mut boa_engine::class::ClassBuilder<'_>) -> JsResult<()> {
    Ok(())
  }

  /// https://developer.mozilla.org/en-US/docs/Web/API/Headers/Headers
  ///
  /// An object containing any HTTP headers that you want to pre-populate your Headers object with.
  /// This can be a simple object literal with String values, an array of name-value pairs, where each pair is a 2-element string array; or an existing Headers object.
  /// In the last case, the new Headers object copies its data from the existing Headers object.
  fn data_constructor(
    _new_target: &JsValue,
    args: &[JsValue],
    context: &mut Context,
  ) -> JsResult<Self> {
    let binding = JsValue::undefined();
    let arg = args.get(0).unwrap_or(&binding);

    if arg.is_object() {
      let binding = arg.to_json(context).unwrap();
      return Ok(Headers::from(binding));
    }

    // Todo: Headers object
    Ok(Headers::from(HeaderMap::new()))
  }
}
