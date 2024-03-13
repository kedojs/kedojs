use boa_engine::{js_string, JsError, JsNativeError, JsResult, JsValue};
use std::{future::poll_fn, path::Path};

use crate::context::KedoContext;

pub struct Kedo {
    context: KedoContext,
}

impl Kedo {
    pub fn new() -> Self {
        let context = KedoContext::new();

        Self { context }
    }

    // Execute a file of JavaScript code
    pub async fn execute(&mut self, str_path: &str) -> Result<JsResult<JsValue>, JsError> {
        let path = Path::new(str_path).canonicalize().map_err(|err| {
            JsNativeError::typ()
                .with_message(format!("Could not canonicalize path `{}`", str_path))
                .with_cause(JsError::from_opaque(js_string!(err.to_string()).into()))
        })?;

        self.context.add_runtime();
        let result = self.context.evaluate(path);
        poll_fn(|_cx| self.context.check_pending_jobs(_cx)).await;
        match result  {
            Ok(_) => Ok(result),
            Err(e) => Err(e),
        }
    }
}
