use std::time::Duration;

use rust_jsc::{
    callback, JSArrayBuffer, JSContext, JSFunction, JSObject, JSResult, JSValue,
};

use crate::{
    context::downcast_state,
    http::fetch_client::fetch_exports,
    job::AsyncJobQueue,
    module::{ModuleEvaluate, ModuleEvaluateDef},
    signals::signal_exports,
    streams::streams::readable_stream_exports,
    timer_queue::{TimerJsCallable, TimerType},
};

use super::{text_encoding::encoding_exports, url_utils::url_exports};

#[callback]
pub fn is_array_buffer_detached(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let object = args[0].as_object().unwrap();
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
    let state = downcast_state::<AsyncJobQueue>(&ctx);
    let function = args[0].as_object()?;
    let function = JSFunction::from(function);
    let timeout = args[1].as_number()? as u64;
    let args = args[2..].to_vec();
    function.protect();
    let id = state.timers().add_timer(
        Duration::from_millis(timeout),
        TimerType::Timeout,
        TimerJsCallable {
            callable: function,
            args,
        },
        Some(true),
    );

    Ok(JSValue::number(&ctx, id as f64))
}

pub fn util_exports(ctx: &JSContext) -> JSObject {
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
        .unwrap();
    exports
        .set_property(
            "is_array_buffer_detached",
            &is_array_buffer_detached_fn,
            Default::default(),
        )
        .unwrap();

    url_exports(ctx, &exports);
    encoding_exports(ctx, &exports);
    readable_stream_exports(ctx, &exports);
    fetch_exports(ctx, &exports);
    signal_exports(ctx, &exports);

    exports
}

pub struct UtilsModule;

impl ModuleEvaluate for UtilsModule {
    fn evaluate(&self, ctx: &JSContext, _name: &str) -> JSObject {
        util_exports(ctx)
    }
}

impl ModuleEvaluateDef for UtilsModule {
    fn name(&self) -> &str {
        "@kedo/internal/utils"
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::modules::internal_utils_tests::setup_module_loader;
    use crate::{module::ModuleResolve, tests::test_utils::new_runtime};
    use rust_jsc::JSContext;

    #[test]
    fn test_utils_module() {
        let ctx = JSContext::new();
        let module = UtilsModule;
        let exports = module.evaluate(&ctx, "@kedo/internal/utils");
        assert!(exports.has_property("is_array_buffer_detached"));
    }

    struct UtilsResolver;

    impl ModuleResolve for UtilsResolver {
        fn resolve(&self, name: &str) -> String {
            name.to_string()
        }

        fn pattern(&self) -> &str {
            "@kedo/internal/utils"
        }
    }

    #[test]
    fn test_utils_module_loader() {
        let ctx = setup_module_loader();
        let result = ctx.evaluate_module_from_source(
            r#"
            import { is_array_buffer_detached } from '@kedo/internal/utils';
            globalThis.is_detached = is_array_buffer_detached(new ArrayBuffer(10));
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = ctx.evaluate_script("globalThis.is_detached", None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_boolean());
        assert_eq!(result.as_boolean(), false);

        let result = ctx.evaluate_module_from_source(
            r#"
            import { is_array_buffer_detached } from '@kedo/internal/utils';
            const buffer = new ArrayBuffer(10);
            const b = buffer.transfer(20);
            globalThis.is_detached = is_array_buffer_detached(buffer);
        "#,
            "index.js",
            None,
        );
        assert!(result.is_ok());
        let result = ctx.evaluate_script("globalThis.is_detached", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_boolean(), true);
    }

    #[tokio::test]
    async fn test_queue_internal_timeout() {
        let mut rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { queue_internal_timeout } from '@kedo/internal/utils';
            globalThis.id = queue_internal_timeout(() => {
                globalThis.done = true;
            }, 100);
        "#,
            "index.js",
            None,
        );
        assert!(result.is_ok());

        let result = rt.evaluate_script("globalThis.id", None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_number());
        let id = result.as_number().unwrap() as u64;
        assert_eq!(id, 1);

        rt.idle().await;

        let result = rt.evaluate_script("globalThis.done", None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_undefined());
    }

    #[tokio::test]
    async fn test_queue_internal_timeout_with_blocking() {
        let mut rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { queue_internal_timeout } from '@kedo/internal/utils';
            queue_internal_timeout(() => {
                globalThis.done = true;
            }, 100);

            setTimeout(() => {}, 100);
        "#,
            "index.js",
            None,
        );
        assert!(result.is_ok());

        rt.idle().await;

        let result = rt.evaluate_script("globalThis.done", None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_boolean());
        assert_eq!(result.as_boolean(), true);
    }
}
