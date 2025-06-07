use rust_jsc::JSValue;

// Custom TryClone trait
pub trait TryClone: Sized {
    fn try_clone(&self) -> Option<Self>;
}

pub trait TryFromValue<T>: Sized {
    fn from_value(value: JSValue) -> rust_jsc::JSResult<T>;
}

pub trait TryIntoValue: Sized {
    fn into_value(
        self,
        ctx: rust_jsc::JSContext,
    ) -> rust_jsc::JSResult<rust_jsc::JSValue>;
}

pub trait TryFromValueInto<T>: TryFromValue<T> + TryIntoValue {}

pub trait TryFromObject: Sized {
    fn from_object(object: rust_jsc::JSObject) -> rust_jsc::JSResult<Self>;
}

pub trait TryIntoObject: Sized {
    fn into_object(
        self,
        ctx: rust_jsc::JSContext,
    ) -> rust_jsc::JSResult<rust_jsc::JSObject>;
}

pub trait TryFromObjectInto: TryFromObject + TryIntoObject {}
