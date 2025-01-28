use kedo_runtime::{ModuleError, ModuleLoader};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct StdModuleLoader {
    modules: HashSet<String>,
}

impl Default for StdModuleLoader {
    fn default() -> Self {
        let modules = vec![
            "@kedo/assert",
            "@kedo/stream",
            "@kedo/events",
            "@kedo/ds",
            "@kedo/fs",
            "@kedo/utils",
            "@kedo/web",
            "@kedo:int/std/stream",
            "@kedo:int/std/web",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        Self { modules }
    }
}

impl ModuleLoader for StdModuleLoader {
    fn resolve(&self, module: &str) -> Result<String, ModuleError> {
        self.modules
            .get(module)
            .map(|_| module.to_string())
            .ok_or_else(|| ModuleError::NotFound(module.to_string()))
    }

    fn can_handle(&self, module_id: &str) -> bool {
        self.modules.contains(module_id)
    }

    fn load(&self, module: &str) -> Result<String, ModuleError> {
        match module {
            "@kedo/assert" => {
                Ok(include_str!("../build/@std/dist/assert/index.js").to_string())
            }
            "@kedo/stream" => {
                Ok(include_str!("../build/@std/dist/stream/index.js").to_string())
            }
            "@kedo:int/std/stream" => {
                Ok(include_str!("../build/@std/dist/stream/_internals.js").to_string())
            }
            "@kedo/events" => {
                Ok(include_str!("../build/@std/dist/events/index.js").to_string())
            }
            "@kedo/ds" => Ok(include_str!("../build/@std/dist/ds/index.js").to_string()),
            "@kedo/fs" => Ok(include_str!("../build/@std/dist/fs/index.js").to_string()),
            "@kedo/utils" => {
                Ok(include_str!("../build/@std/dist/utils/index.js").to_string())
            }
            "@kedo:int/std/web" => {
                Ok(include_str!("../build/@std/dist/web/_internals.js").to_string())
            }
            "@kedo/web" => {
                Ok(include_str!("../build/@std/dist/web/index.js").to_string())
            }
            _ => return Err(ModuleError::NotFound(module.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kedo_runtime::runtime::Runtime;

    #[test]
    fn test_std_modules() {
        let std_modules = StdModuleLoader::default();
        assert_eq!(std_modules.resolve("@kedo/assert").unwrap(), "@kedo/assert");
        assert_eq!(std_modules.resolve("@kedo/stream").unwrap(), "@kedo/stream");
        assert_eq!(std_modules.resolve("@kedo/events").unwrap(), "@kedo/events");
        assert_eq!(std_modules.resolve("@kedo/ds").unwrap(), "@kedo/ds");
        assert_eq!(std_modules.resolve("@kedo/fs").unwrap(), "@kedo/fs");
        assert_eq!(std_modules.resolve("@kedo/utils").unwrap(), "@kedo/utils");
        assert_eq!(std_modules.resolve("@kedo/web").unwrap(), "@kedo/web");
        assert_eq!(
            std_modules.resolve("@kedo:int/std/stream").unwrap(),
            "@kedo:int/std/stream"
        );
        assert_eq!(
            std_modules.resolve("@kedo:int/std/web").unwrap(),
            "@kedo:int/std/web"
        );
    }

    #[test]
    fn test_std_module_loader() {
        let std_modules = StdModuleLoader::default();
        assert_eq!(
            std_modules.load("@kedo/assert").unwrap(),
            include_str!("../build/@std/dist/assert/index.js")
        );
    }

    #[test]
    #[should_panic]
    fn test_std_module_loader_panic() {
        let std_modules = StdModuleLoader::default();
        std_modules
            .load("@kedo/unknown")
            .expect("This should panic");
    }

    #[test]
    fn test_runtime_state() {
        let runtime = Runtime::new();
        runtime.add_loader(StdModuleLoader::default());

        let result = runtime.evaluate_module_from_source(
            r#"
            import assert from '@kedo/assert';
            assert.ok(true, 'This is a test');
        "#,
            "@kedo/std/index",
            None,
        );
        assert!(result.is_ok());

        let result = runtime.evaluate_module_from_source(
            r#"
            import assert from '@kedo/assert';
            assert.ok(false, 'This is a test');
        "#,
            "@kedo/std/index",
            None,
        );
        assert!(result.is_err());
    }
}
