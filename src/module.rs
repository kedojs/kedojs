use std::collections::HashMap;

use rust_jsc::{
    module_evaluate, module_fetch, module_import_meta, module_resolve, JSContext,
    JSModuleLoader, JSObject, JSStringRetain, JSValue,
};

use crate::{context::downcast_state, job::AsyncJobQueue};

// ModuleLoader trait
pub trait ModuleLoader {
    fn load(&self, name: &str) -> String;
    fn pattern(&self) -> &str;
}

// ModuleResolve trait
pub trait ModuleResolve {
    fn resolve(&self, name: &str) -> String;
    fn pattern(&self) -> &str;
}

// ModuleEvaluate trait
pub trait ModuleEvaluate {
    fn evaluate(&self, ctx: &JSContext, name: &str) -> JSObject;
}

pub trait ModuleEvaluateDef: ModuleEvaluate {
    fn name(&self) -> &str;
}

pub type ModuleImportMetaFn = Box<fn(name: &str) -> JSObject>;

pub struct KedoModuleLoader {
    resolvers: Vec<(String, Box<dyn ModuleResolve>)>,
    loaders: Vec<(String, Box<dyn ModuleLoader>)>,

    syn_modules: HashMap<String, Box<dyn ModuleEvaluateDef>>,

    fs_resolver: Option<Box<dyn ModuleResolve>>,
    fs_loader: Option<Box<dyn ModuleLoader>>,
    import_meta: Option<ModuleImportMetaFn>,
    modue_loader: JSModuleLoader,
}

impl KedoModuleLoader {
    pub fn default() -> Self {
        let modue_loader = JSModuleLoader {
            disableBuiltinFileSystemLoader: false,
            moduleLoaderResolve: Some(Self::resolve),
            moduleLoaderEvaluate: Some(Self::evaluate_virtual),
            moduleLoaderFetch: Some(Self::module_loader_fetch),
            moduleLoaderCreateImportMetaProperties: Some(Self::import_meta_properties),
        };

        Self {
            loaders: Vec::new(),
            resolvers: Vec::new(),
            syn_modules: HashMap::new(),

            fs_resolver: None,
            fs_loader: None,
            import_meta: None,
            modue_loader,
        }
    }

    pub fn set_builtin_filesystem_loader(&mut self, enable: bool) {
        self.modue_loader.disableBuiltinFileSystemLoader = !enable;
    }

    pub fn set_import_meta(&mut self, import_meta: ModuleImportMetaFn) {
        self.import_meta = Some(import_meta);
    }

    pub fn set_file_system_loader(&mut self, loader: impl ModuleLoader + 'static) {
        self.fs_loader = Some(Box::new(loader));
        self.modue_loader.disableBuiltinFileSystemLoader = true;
    }

    pub fn init(&self, ctx: &JSContext) {
        let synthenic_keys: Vec<JSStringRetain> =
            self.syn_modules.keys().map(|k| k.as_str().into()).collect();
        ctx.set_virtual_module_keys(synthenic_keys.as_slice());

        ctx.set_module_loader(self.modue_loader.clone());
    }

    pub fn register_loader(&mut self, loader: impl ModuleLoader + 'static) {
        self.loaders
            .push((loader.pattern().to_string(), Box::new(loader)));
        // sort in descending order
        self.loaders.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    }

    pub fn register_resolver(&mut self, resolver: impl ModuleResolve + 'static) {
        self.resolvers
            .push((resolver.pattern().to_string(), Box::new(resolver)));
        // sort in descending order
        self.resolvers.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    }

    pub fn register_synthetic_module(
        &mut self,
        module: impl ModuleEvaluateDef + 'static,
    ) {
        self.syn_modules
            .insert(module.name().to_string(), Box::new(module));
    }

    #[module_resolve]
    fn resolve(
        ctx: JSContext,
        module_name: JSValue,
        _referrer: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringRetain {
        let state = downcast_state::<AsyncJobQueue>(&ctx);
        let key = module_name.as_string().unwrap().to_string();
        let loader = state.module_loader();

        for (pattern, resolver) in loader.resolvers.iter() {
            if key.starts_with(pattern) {
                return resolver.resolve(&key).into();
            }
        }

        if loader.modue_loader.disableBuiltinFileSystemLoader {
            if let Some(fs_resolver) = &loader.fs_resolver {
                return fs_resolver.resolve(&key).into();
            }
        }

        unreachable!("No module resolver found for: {:?}", key);
    }

    #[module_evaluate]
    fn evaluate_virtual(ctx: JSContext, module_name: JSValue) -> JSValue {
        let binding = downcast_state::<AsyncJobQueue>(&ctx);
        let key = module_name
            .as_string()
            .expect("Module name must be a string")
            .to_string();

        if let Some(syn_module) = binding.module_loader().syn_modules.get(&key) {
            return syn_module.evaluate(&ctx, &key).into();
        }

        unreachable!("Module not found: {:?}", key);
    }

    #[module_fetch]
    fn module_loader_fetch(
        ctx: JSContext,
        module_name: JSValue,
        _attributes_value: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringRetain {
        let binding = downcast_state::<AsyncJobQueue>(&ctx);
        let key = module_name.as_string().unwrap().to_string();
        let loader = binding.module_loader();

        for (pattern, loader) in loader.loaders.iter() {
            if key.starts_with(pattern) {
                return loader.load(&key).into();
            }
        }

        if loader.modue_loader.disableBuiltinFileSystemLoader {
            if let Some(fs_loader) = &loader.fs_loader {
                return fs_loader.load(&key).into();
            }
        }

        unreachable!("No module resolver found for: {:?}", key);
    }

    #[module_import_meta]
    fn import_meta_properties(
        ctx: JSContext,
        key: JSValue,
        _script_fetcher: JSValue,
    ) -> JSObject {
        let binding = downcast_state::<AsyncJobQueue>(&ctx);
        if let Some(import_meta) = &binding.module_loader().import_meta {
            import_meta(&key.as_string().unwrap().to_string())
        } else {
            JSObject::new(&ctx)
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{collections::HashSet, path::Path};

    use crate::{
        class_table::ClassTable, proto_table::ProtoTable, timer_queue::TimerQueue,
        RuntimeState,
    };

    use super::*;

    pub struct KedoSyn;

    impl ModuleEvaluate for KedoSyn {
        fn evaluate(&self, ctx: &JSContext, name: &str) -> JSObject {
            let default = JSObject::new(ctx);
            default
                .set_property("name", &JSValue::string(ctx, name), Default::default())
                .unwrap();
            default
        }
    }

    impl ModuleEvaluateDef for KedoSyn {
        fn name(&self) -> &str {
            "@kedo/syn"
        }
    }

    pub struct KedoResolver {
        keys: HashSet<String>,
    }

    impl ModuleResolve for KedoResolver {
        fn resolve(&self, name: &str) -> String {
            if self.keys.contains(name) {
                return name.to_string();
            }

            unreachable!("Module not found: {:?}", name);
        }

        fn pattern(&self) -> &str {
            "@kedo"
        }
    }

    fn new_context_state(loader: KedoModuleLoader) -> RuntimeState<AsyncJobQueue> {
        let timer_queue = TimerQueue::new();
        let job_queue = AsyncJobQueue::new();
        let class_table = ClassTable::new();
        let proto_table = ProtoTable::new();

        let state =
            RuntimeState::new(job_queue, timer_queue, class_table, proto_table, loader);

        state
    }

    #[test]
    fn test_synthetic_module() {
        let mut loader = KedoModuleLoader::default();
        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rust/modules");
        let module_path = format!("{}/01_module.js", module_test_dir);

        let syn_module = KedoSyn {};
        let std_resolver = KedoResolver {
            keys: vec!["@kedo/syn".to_string()].into_iter().collect(),
        };

        loader.register_resolver(std_resolver);
        loader.register_synthetic_module(syn_module);

        let ctx = JSContext::new();
        loader.init(&ctx);

        let state = new_context_state(loader);
        ctx.set_shared_data(Box::new(state));

        let result = ctx.evaluate_module(module_path.as_str());
        assert!(result.is_ok());

        let result = ctx.evaluate_script(
            r"
            globalThis['@kedo/syn']
        ",
            None,
        );

        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        let name = obj.get_property("name").unwrap();
        assert!(name.is_string());
        assert_eq!(name.as_string().unwrap(), "@kedo/syn");
    }

    #[test]
    fn test_module_loader() {
        let mut loader = KedoModuleLoader::default();
        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rust/modules");
        let module_path = format!("{}/02_module.js", module_test_dir);

        struct KedoTestLoader;

        impl ModuleLoader for KedoTestLoader {
            fn load(&self, _name: &str) -> String {
                return r#"
                    export const name = 'Kedo';
                    export const version = '0.1.0';

                    export default {
                        name,
                        version,
                    };
                "#
                .to_string();
            }

            fn pattern(&self) -> &str {
                "@kedo:"
            }
        }

        let kedo_loader = KedoTestLoader {};
        loader.register_loader(kedo_loader);

        struct MockLoader;

        impl ModuleLoader for MockLoader {
            fn load(&self, _name: &str) -> String {
                return r#"
                    export const name = 'Mock';
                    export const version = '1.0.0';

                    export default {
                        name,
                        version,
                    };
                "#
                .to_string();
            }

            fn pattern(&self) -> &str {
                "@mock:"
            }
        }

        let mock_loader = MockLoader {};
        loader.register_loader(mock_loader);

        struct CommonResolver;

        impl ModuleResolve for CommonResolver {
            fn resolve(&self, name: &str) -> String {
                return name.to_string();
            }

            fn pattern(&self) -> &str {
                "@"
            }
        }

        let common_resolver = CommonResolver {};
        loader.register_resolver(common_resolver);

        let ctx = JSContext::new();
        loader.init(&ctx);

        let state = new_context_state(loader);
        ctx.set_shared_data(Box::new(state));

        let result = ctx.evaluate_module(module_path.as_str());

        assert!(result.is_ok());

        let result = ctx.evaluate_script(
            r"
            globalThis['@kedo:']
        ",
            None,
        );

        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        let name = obj.get_property("name").unwrap();
        assert!(name.is_string());
        assert_eq!(name.as_string().unwrap(), "Kedo");

        let result = ctx.evaluate_script(
            r"
            globalThis['@mock:']
        ",
            None,
        );

        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        let name = obj.get_property("name").unwrap();
        assert!(name.is_string());
        assert_eq!(name.as_string().unwrap(), "Mock");
    }

    #[test]
    fn test_file_system_loader() {
        let mut loader = KedoModuleLoader::default();
        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rust/modules");
        let module_path = format!("{}/03_module.js", module_test_dir);

        struct FsLoader;

        impl ModuleLoader for FsLoader {
            fn load(&self, name: &str) -> String {
                let content = std::fs::read_to_string(name).expect("Failed to read file");
                content
            }

            fn pattern(&self) -> &str {
                "file:"
            }
        }

        let file_system_loader = FsLoader {};
        loader.register_loader(file_system_loader);

        struct FsResolver;

        impl ModuleResolve for FsResolver {
            fn resolve(&self, name: &str) -> String {
                let module_test_dir =
                    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rust/modules");
                let path = Path::new(module_test_dir).join(name);
                std::fs::canonicalize(path)
                    .expect("Failed to resolve path")
                    .to_str()
                    .unwrap()
                    .to_string()
            }

            fn pattern(&self) -> &str {
                "file:"
            }
        }

        let file_system_resolver = FsResolver {};
        loader.register_resolver(file_system_resolver);

        let ctx = JSContext::new();
        loader.init(&ctx);

        let state = new_context_state(loader);
        ctx.set_shared_data(Box::new(state));

        let result = ctx.evaluate_module(module_path.as_str());

        assert!(result.is_ok());

        let result = ctx.evaluate_script(
            r"
            globalThis.addOne(4)
        ",
            None,
        );
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_number());
        assert_eq!(result.as_number().unwrap(), 5.0);

        let result = ctx.evaluate_script(
            r"
            globalThis.divTwo(4)
        ",
            None,
        );
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_number());
        assert_eq!(result.as_number().unwrap(), 2.0);

        let result = ctx.evaluate_script(
            r"
            globalThis.mean([1, 2, 3, 4, 5])
        ",
            None,
        );
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_number());
        assert_eq!(result.as_number().unwrap(), 3.0);
    }
}
