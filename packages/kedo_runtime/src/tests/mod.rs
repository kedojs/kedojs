#[cfg(test)]
pub mod test_utils {
    use crate::runtime::Runtime;
    use kedo_core::{AsyncJobQueue, ClassTable, CoreModuleLoader, CoreState, ProtoTable};
    use kedo_std::TimerQueue;

    pub fn new_context_state_with(loader: CoreModuleLoader) -> CoreState {
        let timer_queue = TimerQueue::new();
        let job_queue = AsyncJobQueue::new();
        let class_table = ClassTable::new();
        let proto_table = ProtoTable::new();

        let state =
            CoreState::new(job_queue, timer_queue, class_table, proto_table, loader);

        state
    }

    pub fn new_runtime() -> Runtime {
        Runtime::new()
    }
}
