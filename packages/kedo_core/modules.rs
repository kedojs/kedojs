use crate::state::downcast_state;
use rust_jsc::{
    module_evaluate, module_fetch, module_import_meta, module_resolve, JSContext,
    JSModuleLoader, JSObject, JSStringRetain, JSValue,
};
use std::collections::HashMap;

#[macro_export]
macro_rules! define_exports {
    // Separate template objects and functions
    ($struct_name:ident, @template[$($template:ident),* $(,)?], @function[$($func:ident),* $(,)?]) => {
        impl $struct_name {
            pub fn export(ctx: &JSContext, exports: &JSObject) -> JSResult<()> {
                // Handle template objects
                $(
                    $template::template_object(ctx, exports)?;
                )*

                // Handle functions
                $(
                    let callback_fn = rust_jsc::JSFunction::callback(
                        ctx,
                        Some(stringify!($func)),
                        Some($func),
                    );
                    exports.set_property(
                        stringify!($func),
                        &callback_fn,
                        Default::default(),
                    )?;
                )*

                Ok(())
            }
        }
    };
}

pub type ModuleImportMetaFn = Box<fn(name: &str) -> JSObject>;

// Error handling
#[derive(Debug)]
pub enum ModuleError {
    NotFound(String),
    LoadError(String),
    InvalidModule(String),
    Other(String),
}

// Unified Module Source trait
pub trait ModuleLoader {
    /// Returns true if this source can handle the given module ID
    fn can_handle(&self, module_id: &str) -> bool;
    /// Resolves the module ID
    /// This is used to resolve the module ID to a canonical form
    fn resolve(&self, module_id: &str) -> Result<String, ModuleError>;
    /// Resolves and loads the module content
    /// This is used to resolve and load the module content
    fn load(&self, module_id: &str) -> Result<String, ModuleError>;
}

// Unified Module Evaluate trait
pub trait ModuleSource {
    /// Returns the value of the module
    /// This is used to evaluate the module content
    fn evaluate(&self, ctx: &JSContext, module_id: &str) -> JSObject;
    /// Returns the value of the module
    fn name(&self) -> &str;
}

pub struct CoreModuleLoader {
    loaders: Vec<Box<dyn ModuleLoader>>,
    sources: HashMap<String, Box<dyn ModuleSource>>,
    fs_loader: Option<Box<dyn ModuleLoader>>,
    import_meta: Option<ModuleImportMetaFn>,
    module_loader: JSModuleLoader,
}

impl CoreModuleLoader {
    pub fn default() -> Self {
        let module_loader = JSModuleLoader {
            disableBuiltinFileSystemLoader: false,
            moduleLoaderResolve: Some(Self::resolve),
            moduleLoaderEvaluate: Some(Self::evaluate_virtual),
            moduleLoaderFetch: Some(Self::module_loader_fetch),
            moduleLoaderCreateImportMetaProperties: Some(Self::import_meta_properties),
        };

        Self {
            loaders: Vec::new(),
            sources: HashMap::new(),
            fs_loader: None,
            import_meta: None,
            module_loader,
        }
    }

    pub fn disable_builtin_fs_loader(&mut self) {
        self.module_loader.disableBuiltinFileSystemLoader = true;
    }

    pub fn enable_builtin_fs_loader(&mut self) {
        self.module_loader.disableBuiltinFileSystemLoader = false;
    }

    pub fn set_import_meta(&mut self, import_meta: ModuleImportMetaFn) {
        self.import_meta = Some(import_meta);
    }

    pub fn set_file_system_loader(&mut self, loader: impl ModuleLoader + 'static) {
        self.fs_loader = Some(Box::new(loader));
        self.module_loader.disableBuiltinFileSystemLoader = true;
    }

    pub fn init(&self, ctx: &JSContext) {
        let synthenic_keys: Vec<JSStringRetain> =
            self.sources.keys().map(|k| k.as_str().into()).collect();

        ctx.set_virtual_module_keys(synthenic_keys.as_slice());
        ctx.set_module_loader(self.module_loader.clone());
    }

    pub fn add_loader(&mut self, loader: impl ModuleLoader + 'static) {
        self.loaders.push(Box::new(loader));
    }

    pub fn add_source(&mut self, source: impl ModuleSource + 'static) {
        self.sources
            .insert(source.name().to_string(), Box::new(source));
    }

    #[module_resolve]
    fn resolve(
        ctx: JSContext,
        module_name: JSValue,
        _referrer: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringRetain {
        let state = downcast_state(&ctx);
        let module_loader = state.module_loader().borrow();
        let module_id = module_name
            .as_string()
            .expect("Module name must be a string")
            .to_string();

        if let Some(_) = module_loader.sources.get(&module_id) {
            return module_id.into();
        }

        for loader in module_loader.loaders.iter() {
            if loader.can_handle(&module_id) {
                return loader.resolve(&module_id).unwrap().into();
            }
        }

        if module_loader.module_loader.disableBuiltinFileSystemLoader {
            if let Some(fs_loader) = &module_loader.fs_loader {
                return fs_loader.resolve(&module_id).unwrap().into();
            }
        }

        unreachable!("Invalid Module: {:?}", module_id);
    }

    #[module_evaluate]
    fn evaluate_virtual(ctx: JSContext, module_name: JSValue) -> JSValue {
        let binding = downcast_state(&ctx);
        let loader = binding.module_loader().borrow();
        let module_id = module_name
            .as_string()
            .expect("Module name must be a string")
            .to_string();

        if let Some(source) = loader.sources.get(&module_id) {
            return source.evaluate(&ctx, &module_id).into();
        }

        unreachable!("Module: {:?} not found", module_id);
    }

    #[module_fetch]
    fn module_loader_fetch(
        ctx: JSContext,
        module_name: JSValue,
        _attributes_value: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringRetain {
        let binding = downcast_state(&ctx);
        let loader = binding.module_loader().borrow();
        let module_id = module_name
            .as_string()
            .expect("Module name must be a string")
            .to_string();

        for loader in loader.loaders.iter() {
            if loader.can_handle(&module_id) {
                return loader.load(&module_id).unwrap().into();
            }
        }

        if loader.module_loader.disableBuiltinFileSystemLoader {
            if let Some(fs_loader) = &loader.fs_loader {
                return fs_loader.load(&module_id).unwrap().into();
            }
        }

        unreachable!("No module found for: {:?}", module_id);
    }

    #[module_import_meta]
    fn import_meta_properties(
        ctx: JSContext,
        key: JSValue,
        _script_fetcher: JSValue,
    ) -> JSObject {
        let binding = downcast_state(&ctx);
        let loader = binding.module_loader().borrow();
        if let Some(import_meta) = &loader.import_meta {
            import_meta(&key.as_string().unwrap().to_string())
        } else {
            JSObject::new(&ctx)
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{collections::HashSet, path::Path};

    use kedo_std::TimerQueue;
    use rust_jsc::{JSContext, JSObject, JSValue};

    use crate::{
        class_table::ClassTable, job::AsyncJobQueue, proto_table::ProtoTable,
        state::CoreState,
    };

    use super::*;

    pub struct KedoSyn;

    impl ModuleSource for KedoSyn {
        fn evaluate(&self, ctx: &JSContext, name: &str) -> JSObject {
            let default = JSObject::new(ctx);
            let exports = JSObject::new(ctx);
            exports
                .set_property("name", &JSValue::string(ctx, name), Default::default())
                .unwrap();

            default
                .set_property("default", &exports, Default::default())
                .unwrap();
            default
        }

        fn name(&self) -> &str {
            "@kedo/syn"
        }
    }

    pub struct KedoResolver {
        keys: HashSet<String>,
    }

    impl ModuleLoader for KedoResolver {
        fn resolve(&self, name: &str) -> Result<String, ModuleError> {
            if self.keys.contains(name) {
                return Ok(name.to_string());
            }

            Err(ModuleError::NotFound(name.to_string()))
        }

        fn can_handle(&self, module_id: &str) -> bool {
            self.keys.contains(module_id)
        }

        fn load(&self, _module_id: &str) -> Result<String, ModuleError> {
            Ok("".to_string())
        }
    }

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
    fn test_synthetic_module() {
        let mut loader = CoreModuleLoader::default();
        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        let module_path = format!("{}/01_module.js", module_test_dir);

        let syn_module = KedoSyn {};
        let std_resolver = KedoResolver {
            keys: vec!["@kedo/syn".to_string()].into_iter().collect(),
        };

        loader.add_loader(std_resolver);
        loader.add_source(syn_module);

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
        let mut loader = CoreModuleLoader::default();
        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        let module_path = format!("{}/02_module.js", module_test_dir);

        struct KedoTestLoader;

        impl ModuleLoader for KedoTestLoader {
            fn load(&self, _name: &str) -> Result<String, ModuleError> {
                return Ok(r#"
                    export const name = 'Kedo';
                    export const version = '0.1.0';

                    export default {
                        name,
                        version,
                    };
                "#
                .to_string());
            }

            fn can_handle(&self, module_id: &str) -> bool {
                module_id.starts_with("@kedo:")
            }

            fn resolve(&self, name: &str) -> Result<String, ModuleError> {
                Ok(name.to_string())
            }
        }

        let kedo_loader = KedoTestLoader {};
        loader.add_loader(kedo_loader);

        struct MockLoader;

        impl ModuleLoader for MockLoader {
            fn load(&self, _name: &str) -> Result<String, ModuleError> {
                return Ok(r#"
                    export const name = 'Mock';
                    export const version = '1.0.0';

                    export default {
                        name,
                        version,
                    };
                "#
                .to_string());
            }

            fn can_handle(&self, module_id: &str) -> bool {
                module_id.starts_with("@mock:")
            }

            fn resolve(&self, name: &str) -> Result<String, ModuleError> {
                Ok(name.to_string())
            }
        }

        let mock_loader = MockLoader {};
        loader.add_loader(mock_loader);

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
        let mut loader = CoreModuleLoader::default();
        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        let module_path = format!("{}/03_module.js", module_test_dir);

        struct FsLoader;

        impl ModuleLoader for FsLoader {
            fn load(&self, name: &str) -> Result<String, ModuleError> {
                let content = std::fs::read_to_string(name);
                match content {
                    Ok(content) => Ok(content),
                    Err(_) => Err(ModuleError::LoadError(name.to_string())),
                }
            }

            fn can_handle(&self, module_id: &str) -> bool {
                module_id.starts_with("file:")
            }

            fn resolve(&self, name: &str) -> Result<String, ModuleError> {
                let module_test_dir =
                    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rust/modules");
                let path = Path::new(module_test_dir).join(name);
                let path = std::fs::canonicalize(path)
                    .map_err(|_| ModuleError::NotFound(name.to_string()))?
                    .to_str()
                    .ok_or(ModuleError::InvalidModule(name.to_string()))?
                    .to_string();

                Ok(path)
            }
        }

        let file_system_loader = FsLoader {};
        loader.add_loader(file_system_loader);

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
