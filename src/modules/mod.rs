use crate::module::ModuleResolve;

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
