#[derive(Clone)]
pub struct JsProctectedCallable {
    pub callable: rust_jsc::JSFunction,
    pub args: Vec<rust_jsc::JSValue>,
}

impl Drop for JsProctectedCallable {
    fn drop(&mut self) {
        self.callable.unprotect();
    }
}

impl JsProctectedCallable {
    pub fn new(callable: rust_jsc::JSFunction, args: Vec<rust_jsc::JSValue>) -> Self {
        callable.protect();
        Self { callable, args }
    }

    pub fn call(&self) -> rust_jsc::JSResult<rust_jsc::JSValue> {
        self.callable.call(None, &self.args.as_slice())
    }
}
