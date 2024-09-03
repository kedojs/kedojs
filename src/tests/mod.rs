#[cfg(test)]
pub mod test_utils {
    use crate::{
        class_table::ClassTable, job::AsyncJobQueue, module::KedoModuleLoader,
        proto_table::ProtoTable, runtime::Runtime, timer_queue::TimerQueue, RuntimeState,
    };

    pub fn new_context_state_with(
        loader: KedoModuleLoader,
    ) -> RuntimeState<AsyncJobQueue> {
        let timer_queue = TimerQueue::new();
        let job_queue = AsyncJobQueue::new();
        let class_table = ClassTable::new();
        let proto_table = ProtoTable::new();

        let state =
            RuntimeState::new(job_queue, timer_queue, class_table, proto_table, loader);

        state
    }

    pub fn new_runtime() -> Runtime {
        Runtime::new()
    }
}
