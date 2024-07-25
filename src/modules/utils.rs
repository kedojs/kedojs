use rust_jsc::{
    callback, JSArrayBuffer, JSContext, JSFunction, JSObject, JSResult, JSValue,
};

use crate::module::{ModuleEvaluate, ModuleEvaluateDef};

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

pub fn util_exports(ctx: &JSContext) -> JSObject {
    let export = JSObject::new(ctx);

    let function =
        JSFunction::callback(&ctx, Some("is_detached"), Some(is_array_buffer_detached));
    export
        .set_property("is_array_buffer_detached", &function, Default::default())
        .unwrap();
    export
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
    use core::str;

    use rust_jsc::JSContext;

    use crate::{
        class_table::ClassTable, job::AsyncJobQueue, module::{KedoModuleLoader, ModuleResolve},
        proto_table::ProtoTable, timer_queue::TimerQueue, RuntimeState,
    };

    use super::*;

    #[test]
    fn test_utils_module() {
        let ctx = JSContext::new();
        let module = UtilsModule;
        let exports = module.evaluate(&ctx, "@kedo/internal/utils");
        assert!(exports.has_property("is_array_buffer_detached"));
    }

    // #[test]
    // fn test_synthetic_module() {
    //     let mut loader = KedoModuleLoader::default();
    //     let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rust/modules");
    //     let module_path = format!("{}/01_module.js", module_test_dir);

    //     let syn_module = KedoSyn {};
    //     let std_resolver = KedoResolver {
    //         keys: vec!["@kedo/syn".to_string()].into_iter().collect(),
    //     };

    //     loader.register_resolver(std_resolver);
    //     loader.register_synthetic_module(syn_module);

    //     let ctx = JSContext::new();
    //     loader.init(&ctx);

    //     let state = new_context_state(loader);
    //     ctx.set_shared_data(Box::new(state));

    //     let result = ctx.evaluate_module(module_path.as_str());
    //     assert!(result.is_ok());

    //     let result = ctx.evaluate_script(
    //         r"
    //         globalThis['@kedo/syn']
    //     ",
    //         None,
    //     );

    //     assert!(result.is_ok());

    //     let result = result.unwrap();
    //     assert!(result.is_object());
    //     let obj = result.as_object().unwrap();
    //     let name = obj.get_property("name").unwrap();
    //     assert!(name.is_string());
    //     assert_eq!(name.as_string().unwrap(), "@kedo/syn");
    // }

    fn new_context_state(loader: KedoModuleLoader) -> RuntimeState<AsyncJobQueue> {
        let timer_queue = TimerQueue::new();
        let job_queue = AsyncJobQueue::new();
        let class_table = ClassTable::new();
        let proto_table = ProtoTable::new();

        let state =
            RuntimeState::new(job_queue, timer_queue, class_table, proto_table, loader);

        state
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
        let mut loader = KedoModuleLoader::default();
        loader.register_resolver(UtilsResolver);
        loader.register_synthetic_module(UtilsModule);

        let ctx = JSContext::new();
        loader.init(&ctx);

        let state = new_context_state(loader);
        ctx.set_shared_data(Box::new(state));

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
}
