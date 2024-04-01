use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::task::Poll;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::context::{ContextBuilder, HostHooks};
use boa_engine::realm::Realm;
use boa_engine::{js_string, property::Attribute, Context, JsResult, JsValue, Source};
use boa_engine::{JsError, JsNativeError, JsObject, JsString, Module, NativeFunction};
use boa_runtime::Console;

use crate::file::FileSystem;
use crate::file_dir::KedoDirEntry;
use crate::http;
use crate::http::headers::Headers;
use crate::timer::{Timer, TimerQueue};

struct Hooks;

impl HostHooks for Hooks {
  fn ensure_can_compile_strings(
    &self,
    _realm: Realm,
    _parameters: &[JsString],
    _body: &JsString,
    _direct: bool,
    _context: &mut Context,
  ) -> JsResult<()> {
    Err(
      JsNativeError::typ()
        .with_message("eval calls not available")
        .into(),
    )
  }

  fn promise_rejection_tracker(
    &self,
    _promise: &boa_engine::prelude::JsObject,
    _operation: boa_engine::builtins::promise::OperationType,
    _context: &mut Context,
  ) {
    // todo!("Implement onRejectionHandled and onUnhandledRejection hooks");
  }
}

pub struct KedoContext {
  context: Context,
  timers: Rc<RefCell<TimerQueue>>,
  module_loader: Rc<boa_engine::module::SimpleModuleLoader>,
}

impl KedoContext {
  pub const NAME: &'static str = "Kedo";
  // pub const GLOBAL_THIS: &'static str = "globalThis";

  pub fn new() -> Self {
    let module_loader =
      Rc::new(boa_engine::module::SimpleModuleLoader::new(".").unwrap());

    let context = ContextBuilder::new()
      .host_hooks(&Hooks)
      .module_loader(module_loader.clone())
      .build()
      .expect("Failed to create context");

    let timers = Rc::new(RefCell::new(TimerQueue::new()));
    Self {
      context,
      timers,
      module_loader,
    }
  }

  pub fn evaluate(&mut self, path: PathBuf) -> JsResult<JsValue> {
    let source = match Source::from_filepath(&path) {
      Ok(src) => src,
      Err(e) => {
        return Err(JsNativeError::typ().with_message(e.to_string()).into());
      }
    };

    let module = Module::parse(source, None, &mut self.context)?;
    self
      .module_loader
      .insert(path.to_path_buf(), module.clone());

    let promise_result = module
      // Initial load that recursively loads the module's dependencies.
      // This returns a `JsPromise` that will be resolved when loading finishes,
      // which allows async loads and async fetches.
      .load(&mut self.context)
      .then(
        Some(
          NativeFunction::from_copy_closure_with_captures(
            |_, _, module, context| {
              // After loading, link all modules by resolving the imports
              // and exports on the full module graph, initializing module
              // environments. This returns a plain `Err` since all modules
              // must link at the same time.
              module.link(context)?;
              Ok(JsValue::undefined())
            },
            module.clone(),
          )
          .to_js_function(self.context.realm()),
        ),
        None,
        &mut self.context,
      )
      .then(
        Some(
          NativeFunction::from_copy_closure_with_captures(
            // Finally, evaluate the root module.
            // This returns a `JsPromise` since a module could have
            // top-level await statements, which defers module execution to the
            // job queue.
            |_, _, module, context| Ok(module.evaluate(context).into()),
            module.clone(),
          )
          .to_js_function(self.context.realm()),
        ),
        None,
        &mut self.context,
      );

    self.context.run_jobs();

    // Checking if the final promise didn't return an error.
    match promise_result.state() {
      PromiseState::Pending => {
        return Err(
          JsNativeError::typ()
            .with_message("Module didn't execute!")
            .into(),
        )
      }
      PromiseState::Fulfilled(v) => {
        assert_eq!(v, JsValue::undefined());
      }
      PromiseState::Rejected(err) => {
        return Err(
          JsError::from_opaque(err)
            .try_native(&mut self.context)
            .unwrap()
            .into(),
        )
      }
    }

    let namespace = module.namespace(&mut self.context);
    let result = namespace.get(js_string!("result"), &mut self.context)?;

    Ok(result)
  }

  pub fn add_runtime(&mut self) {
    self.register_console();
    self.register_global()
  }

  pub fn check_pending_jobs(&mut self, cx: &mut std::task::Context) -> Poll<()> {
    let result = self.timers.borrow_mut().poll_timers(cx, &mut self.context);
    self.context.run_jobs();

    if let Poll::Ready(_) = result {
      let res = self.timers.borrow_mut().poll_timers(cx, &mut self.context);
      self.context.run_jobs();
      return res;
    }

    result
  }

  fn register_console(&mut self) {
    let console = Console::init(&mut self.context);
    self
      .context
      .register_global_property(js_string!(Console::NAME), console, Attribute::all())
      .expect("the console builtin shouldn't exist");
  }

  fn register_global(&mut self) {
    let kedo_object = JsObject::with_object_proto(self.context.intrinsics());

    Timer::register_timers(&mut self.context, self.timers.clone());
    http::init_with_object(&mut self.context, &kedo_object).unwrap();
    FileSystem::init_with_object(&mut self.context, &kedo_object).unwrap();

    self
      .context
      .register_global_property(js_string!(Self::NAME), kedo_object, Attribute::all())
      .expect("the file builtin shouldn't exist");

    self
      .context
      .register_global_class::<KedoDirEntry>()
      .expect("the KedoDirEntry builtin shouldn't exist");
    self
      .context
      .register_global_class::<Headers>()
      .expect("the Timer builtin shouldn't exist");
  }
}
