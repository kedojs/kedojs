use std::mem::ManuallyDrop;

use rust_jsc::{JSObject, PrivateData};

use crate::ManuallyDropClone;

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
