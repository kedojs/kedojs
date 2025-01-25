use kedo_core::{ModuleLoader, ModuleResolve};

const PATTERN: &'static str = "@kedo";

#[derive(Debug, Clone)]
pub struct StdModuleResolver;

impl ModuleResolve for StdModuleResolver {
    fn resolve(&self, module: &str) -> String {
        match module {
            "@kedo/assert" => module.to_string(),
            "@kedo/stream" => module.to_string(),
            "@kedo/events" => module.to_string(),
            "@kedo/ds" => module.to_string(),
            "@kedo/fs" => module.to_string(),
            "@kedo/utils" => module.to_string(),
            "@kedo/web" => module.to_string(),
            "@kedo:int/std/stream" => module.to_string(),
            "@kedo:int/std/web" => module.to_string(),
            _ => unreachable!("Module not found: {}", module),
        }
    }

    fn pattern(&self) -> &str {
        PATTERN
    }
}

#[derive(Debug, Clone)]
pub struct StdModuleLoader;

impl ModuleLoader for StdModuleLoader {
    fn load(&self, module: &str) -> String {
        match module {
            "@kedo/assert" => {
                include_str!("../../../build/@std/dist/assert/index.js").to_string()
            }
            "@kedo/stream" => {
                include_str!("../../../build/@std/dist/stream/index.js").to_string()
            }
            "@kedo:int/std/stream" => {
                include_str!("../../../build/@std/dist/stream/_internals.js").to_string()
            }
            "@kedo/events" => {
                include_str!("../../../build/@std/dist/events/index.js").to_string()
            }
            "@kedo/ds" => {
                include_str!("../../../build/@std/dist/ds/index.js").to_string()
            }
            "@kedo/fs" => {
                include_str!("../../../build/@std/dist/fs/index.js").to_string()
            }
            "@kedo/utils" => {
                include_str!("../../../build/@std/dist/utils/index.js").to_string()
            }
            "@kedo:int/std/web" => {
                include_str!("../../../build/@std/dist/web/_internals.js").to_string()
            }
            "@kedo/web" => {
                include_str!("../../../build/@std/dist/web/index.js").to_string()
            }
            _ => unreachable!("Module not found: {}", module),
        }
    }

    fn pattern(&self) -> &str {
        PATTERN
    }
}

#[cfg(test)]
mod tests {
    use kedo_core::{AsyncJobQueue, ClassTable, CoreModuleLoader, CoreState, ProtoTable};
    use kedo_std::TimerQueue;
    use rust_jsc::JSContext;

    use super::*;

    fn new_context_state(loader: CoreModuleLoader) -> CoreState {
        let timer_queue = TimerQueue::new();
        let job_queue = AsyncJobQueue::new();
        let class_table = ClassTable::new();
        let proto_table = ProtoTable::new();

        let state =
            CoreState::new(job_queue, timer_queue, class_table, proto_table, loader);

        state
    }

    #[test]
    fn test_std_modules() {
        let std_modules = StdModuleResolver;
        assert_eq!(std_modules.pattern(), "@kedo");
        assert_eq!(std_modules.resolve("@kedo/assert"), "@kedo/assert");
    }

    #[test]
    fn test_std_module_loader() {
        let std_module_loader = StdModuleLoader;
        assert_eq!(std_module_loader.pattern(), "@kedo");
        assert_eq!(
            std_module_loader.load("@kedo/assert"),
            include_str!("../../../build/@std/dist/assert/index.js")
        );
    }

    #[test]
    fn test_loader_std_index() {
        // let std_module_loader = StdModuleLoader;
        // assert_eq!(std_module_loader.pattern(), "@kedo");
        // assert_eq!(
        //     std_module_loader.load("@kedo/std/index"),
        //     include_str!("@std/index.js")
        // );
    }

    #[test]
    #[should_panic]
    fn test_std_module_loader_panic() {
        let std_module_loader = StdModuleLoader;
        std_module_loader.load("@kedo/unknown");
    }

    #[test]
    #[should_panic]
    fn test_std_modules_resolver_panic() {
        let std_modules = StdModuleResolver;
        std_modules.resolve("@kedo/unknown");
    }

    #[test]
    fn test_runtime_state() {
        let mut loader = CoreModuleLoader::default();
        loader.register_loader(StdModuleLoader);
        loader.register_resolver(StdModuleResolver);

        let ctx = JSContext::new();
        loader.set_builtin_filesystem_loader(true);
        loader.init(&ctx);

        let state = new_context_state(loader);
        ctx.set_shared_data(Box::new(state));

        let result = ctx.evaluate_module_from_source(
            r#"
            import assert from '@kedo/assert';
            assert.ok(true, 'This is a test');
        "#,
            "@kedo/std/index",
            None,
        );
        assert!(result.is_ok());

        let result = ctx.evaluate_module_from_source(
            r#"
            import assert from '@kedo/assert';
            assert.ok(false, 'This is a test');
        "#,
            "@kedo/std/index",
            None,
        );
        assert!(result.is_err());
    }

    // #[test]
    // fn test_runtime_std_index() {
    //     let mut loader = CoreModuleLoader::default();
    //     loader.register_loader(StdModuleLoader);
    //     loader.register_resolver(StdModuleResolver);

    //     let ctx = JSContext::new();
    //     loader.set_builtin_filesystem_loader(true);
    //     loader.init(&ctx);

    //     let state = new_context_state(loader);
    //     ctx.set_shared_data(Box::new(state));

    //     let result = ctx.evaluate_module("@kedo/std/index");
    //     if let Err(e) = result {
    //         panic!("Error: {}", e.message().unwrap());
    //     }
    // }
}
