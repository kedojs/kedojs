use crate::module::ModuleResolve;

pub mod text_decoder_inner;
pub mod text_encoding;
pub mod url_record;
pub mod url_utils;
pub mod utils;

pub struct InternalModuleResolver;

impl ModuleResolve for InternalModuleResolver {
    fn resolve(&self, name: &str) -> String {
        match name {
            "@kedo/internal/utils" => name.to_string(),
            _ => panic!("Module not found: {}", name),
        }
    }

    fn pattern(&self) -> &str {
        "@kedo/internal/"
    }
}

#[cfg(test)]
mod internal_utils_tests {
    use super::*;
    use crate::module::{KedoModuleLoader, ModuleResolve};
    use crate::tests::test_utils::new_context_state_with;
    use rust_jsc::JSContext;
    use utils::UtilsModule;

    struct UtilsResolver;

    impl ModuleResolve for UtilsResolver {
        fn resolve(&self, name: &str) -> String {
            name.to_string()
        }

        fn pattern(&self) -> &str {
            "@kedo/internal/utils"
        }
    }

    pub fn setup_module_loader() -> JSContext {
        let mut loader = KedoModuleLoader::default();
        loader.register_resolver(UtilsResolver);
        loader.register_synthetic_module(UtilsModule);

        let ctx = JSContext::new();
        loader.init(&ctx);

        let state = new_context_state_with(loader);
        ctx.set_shared_data(Box::new(state));
        ctx
    }
}
