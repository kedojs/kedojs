mod callback;
mod class_table;
mod job;
mod modules;
mod proto_table;
mod state;

pub use job::AsyncJobQueue;
pub use job::AsyncJobQueueInner;
pub use job::JobQueue;
pub use job::NativeJob;
pub use job::SimpleJobQueue;

// modules
pub use modules::CoreModuleLoader;
pub use modules::ModuleError;
pub use modules::ModuleImportMetaFn;
pub use modules::ModuleLoader;
pub use modules::ModuleSource;

pub use callback::JsProctectedCallable;
// state
pub use class_table::ClassTable;
pub use proto_table::ProtoTable;
pub use state::downcast_state;
pub use state::CoreState;

use std::io;

use tokio::task::spawn_blocking;

pub async fn asyncify<F, T>(f: F) -> io::Result<T>
where
    F: FnOnce() -> io::Result<T> + Send + 'static,
    T: Send + 'static,
{
    match spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::Other,
            "background task failed",
        )),
    }
}
