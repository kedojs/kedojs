use std::path::{Path, PathBuf};

use boa_engine::{
  js_string,
  module::{ModuleLoader, Referrer},
  Context, JsError, JsNativeError, JsResult, JsString, Module,
};
use boa_gc::GcRefCell;
use boa_parser::Source;
use rustc_hash::FxHashMap;

/// A module loader that loads modules relative to a root path and resolves the standard library
#[derive(Debug)]
pub struct KedoModuleLoader {
  root: PathBuf,
  std_path: PathBuf,
  module_map: GcRefCell<FxHashMap<PathBuf, Module>>,
}

impl KedoModuleLoader {
  /// Creates a new `KedoModuleLoader` from a root module path and a standard library path.
  pub fn new<P: AsRef<Path>>(root: P, std_path: P) -> JsResult<Self> {
    if cfg!(target_family = "wasm") {
      return Err(
        JsNativeError::typ()
          .with_message("cannot resolve a relative path in WASM targets")
          .into(),
      );
    }
    let root = root.as_ref();
    let absolute = root.canonicalize().map_err(|e| {
      JsNativeError::typ()
        .with_message(format!("could not set module root `{}`", root.display()))
        .with_cause(JsError::from_opaque(js_string!(e.to_string()).into()))
    })?;

    let std_path = std_path.as_ref();
    let absolute_std_path = std_path.canonicalize().map_err(|e| {
      JsNativeError::typ()
        .with_message(format!("Could not set std module `{}`", std_path.display()))
        .with_cause(JsError::from_opaque(js_string!(e.to_string()).into()))
    })?;
    Ok(Self {
      root: absolute,
      std_path: absolute_std_path,
      module_map: GcRefCell::default(),
    })
  }

  /// Inserts a new module onto the module map.
  #[inline]
  pub fn insert(&self, path: PathBuf, module: Module) {
    self.module_map.borrow_mut().insert(path, module);
  }

  /// Gets a module from its original path.
  #[inline]
  pub fn get(&self, path: &Path) -> Option<Module> {
    self.module_map.borrow().get(path).cloned()
  }
}

impl ModuleLoader for KedoModuleLoader {
  fn load_imported_module(
    &self,
    _referrer: Referrer,
    specifier: JsString,
    finish_load: Box<dyn FnOnce(JsResult<Module>, &mut Context)>,
    context: &mut Context,
  ) {
    let result = (|| {
      let path = specifier
        .to_std_string()
        .map_err(|err| JsNativeError::typ().with_message(err.to_string()))?;
      let short_path = Path::new(&path);
      let path = self.root.join(short_path);
      let path = path.canonicalize().map_err(|err| {
        JsNativeError::typ()
          .with_message(format!(
            "could not canonicalize path `{}`",
            short_path.display()
          ))
          .with_cause(JsError::from_opaque(js_string!(err.to_string()).into()))
      })?;
      if let Some(module) = self.get(&path) {
        return Ok(module);
      }
      let source = Source::from_filepath(&path).map_err(|err| {
        JsNativeError::typ()
          .with_message(format!("could not open file `{}`", short_path.display()))
          .with_cause(JsError::from_opaque(js_string!(err.to_string()).into()))
      })?;
      let module = Module::parse(source, None, context).map_err(|err| {
        JsNativeError::syntax()
          .with_message(format!("could not parse module `{}`", short_path.display()))
          .with_cause(err)
      })?;
      self.insert(path, module.clone());
      Ok(module)
    })();

    finish_load(result, context);
  }
}
