use crate::{
    encoding::text_encoding::TextEncodingModule,
    http::{
        fetch::FetchModule, request::FetchRequestOps, server::server_exports,
        url_module::UrlModule,
    },
    signals::signal_exports,
    StreamResourceModule,
};
use kedo_core::{downcast_state, JsProctectedCallable, ModuleSource};
use rust_jsc::{
    callback, JSArrayBuffer, JSContext, JSFunction, JSObject, JSResult, JSValue,
};
use std::time::Duration;

#[callback]
pub fn is_array_buffer_detached(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    object: JSObject,
) -> JSResult<JSValue> {
    let array_buffer = JSArrayBuffer::from_object(object);
    Ok(JSValue::boolean(&ctx, array_buffer.is_detached()))
}

#[callback]
pub fn queue_internal_timeout(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let state = downcast_state(&ctx);
    let function = args[0].as_object()?;
    let function = JSFunction::from(function);
    let timeout = args[1].as_number()? as u64;
    let args = args[2..].to_vec();
    function.protect();
    let id = state.timers().add_timer(
        Duration::from_millis(timeout),
        kedo_std::TimerType::Timeout,
        JsProctectedCallable::new(function, args),
        Some(true),
    );

    Ok(JSValue::number(&ctx, id as f64))
}

fn exports(ctx: &JSContext) -> JSObject {
    let exports = JSObject::new(ctx);

    let is_array_buffer_detached_fn =
        JSFunction::callback(&ctx, Some("is_detached"), Some(is_array_buffer_detached));
    let queue_internal_timeout_fn = JSFunction::callback(
        &ctx,
        Some("queue_internal_timeout"),
        Some(queue_internal_timeout),
    );

    exports
        .set_property(
            "queue_internal_timeout",
            &queue_internal_timeout_fn,
            Default::default(),
        )
        .expect("Failed to set queue_internal_timeout");
    exports
        .set_property(
            "is_array_buffer_detached",
            &is_array_buffer_detached_fn,
            Default::default(),
        )
        .expect("Failed to set is_array_buffer_detached");

    UrlModule::export(ctx, &exports).expect("Failed to export UrlUtilModule");
    TextEncodingModule::export(ctx, &exports)
        .expect("Failed to export TextEncodingModule");
    StreamResourceModule::export(ctx, &exports)
        .expect("Failed to export StreamResourceModule");
    FetchModule::export(ctx, &exports).expect("Failed to export FetchModule");
    FetchRequestOps::export(ctx, &exports).expect("Failed to export FetchRequestOps");

    server_exports(ctx, &exports);
    signal_exports(ctx, &exports);

    exports
}

pub struct WebModule;

impl ModuleSource for WebModule {
    fn evaluate(&self, ctx: &JSContext, _name: &str) -> JSObject {
        exports(ctx)
    }

    fn name(&self) -> &str {
        "@kedo:op/web"
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rust_jsc::JSContext;

    #[test]
    fn test_utils_module() {
        let ctx = JSContext::new();
        let module = WebModule;
        let exports = module.evaluate(&ctx, "@kedo:op/web");
        assert!(exports.has_property("is_array_buffer_detached"));
    }
}
