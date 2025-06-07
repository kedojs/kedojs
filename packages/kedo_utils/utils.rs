use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use rust_jsc::{JSContext, JSError, JSObject, PrivateData};

pub struct ManuallyDropArc<T>(ManuallyDrop<Arc<T>>);

impl<T> ManuallyDropArc<T> {
    #[allow(unused)]
    pub fn clone(&self) -> Arc<T> {
        self.0.deref().clone()
    }
}

impl<T> Deref for ManuallyDropArc<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for ManuallyDropArc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

pub struct ManuallyDropClone<T>(ManuallyDrop<T>);

impl<T> ManuallyDropClone<T> {
    #[allow(unused)]
    pub fn clone(&self) -> T
    where
        T: Clone,
    {
        self.0.deref().clone()
    }

    pub fn new(value: T) -> Self {
        ManuallyDropClone(ManuallyDrop::new(value))
    }

    pub fn take(self) -> T {
        ManuallyDrop::into_inner(self.0)
    }
}

impl<T> Deref for ManuallyDropClone<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for ManuallyDropClone<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

pub(crate) struct ManuallyDropRc<T>(ManuallyDrop<Rc<T>>);

impl<T> ManuallyDropRc<T> {
    #[allow(unused)]
    pub fn clone(&self) -> Rc<T> {
        self.0.deref().clone()
    }
}

impl<T> Deref for ManuallyDropRc<T> {
    type Target = Rc<T>;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for ManuallyDropRc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

#[macro_export]
macro_rules! js_error {
    ($ctx:expr, $msg:expr) => {{
        JSError::with_message($ctx, $msg)?
    }};
}

#[macro_export]
macro_rules! js_error_typ {
    ($ctx:expr, $msg:expr) => {{
        JSError::new_typ($ctx, $msg)?
    }};
}

#[macro_export]
macro_rules! js_undefined {
    ($ctx:expr) => {
        JSValue::undefined($ctx)
    };
}

#[macro_export]
macro_rules! js_null {
    ($ctx:expr) => {
        JSValue::null($ctx)
    };
}

pub fn downcast_ref<T>(this: &JSObject) -> Option<ManuallyDropClone<Box<T>>> {
    let data = this.get_private_data::<T>()?;
    Some(ManuallyDropClone(ManuallyDrop::new(data)))
}

pub fn downcast_ptr<T>(data_ptr: PrivateData) -> Option<ManuallyDropClone<Box<T>>> {
    if data_ptr.is_null() {
        return None;
    }

    let data = unsafe { Box::from_raw(data_ptr as *mut T) };
    Some(ManuallyDropClone(ManuallyDrop::new(data)))
}

pub fn upcast<T>(data: Box<T>) -> PrivateData {
    Box::into_raw(data) as *mut T as PrivateData
}

pub fn drop_ptr<T>(data_ptr: PrivateData) {
    if data_ptr.is_null() {
        return;
    }

    unsafe {
        let value = Box::from_raw(data_ptr as *mut T);
        drop(value);
    };
}

pub fn map_err_from_option<T>(ctx: &JSContext, value: Option<T>) -> Result<T, JSError> {
    match value {
        Some(value) => Ok(value),
        None => Err(JSError::new_typ(ctx, "Expected 1 argument")?),
    }
}
