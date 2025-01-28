pub mod text_decoder_inner;
pub mod text_encoding;
pub mod url_record;
pub mod url_utils;
pub mod utils;

#[cfg(test)]
mod internal_utils_tests {
    use super::*;
    use crate::tests::test_utils::new_context_state_with;
    use kedo_core::CoreModuleLoader;
    use rust_jsc::JSContext;
    use utils::UtilsModule;

    pub fn setup_module_loader() -> JSContext {
        let mut loader = CoreModuleLoader::default();
        loader.add_source(UtilsModule);

        let ctx = JSContext::new();
        loader.init(&ctx);

        let state = new_context_state_with(loader);
        ctx.set_shared_data(Box::new(state));
        ctx
    }
}
