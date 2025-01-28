use std::{cell::RefCell, rc::Rc, sync::Arc};

use kedo_std::TimerQueue;
use kedo_utils::ManuallyDropClone;

use crate::{
    callback::JsProctectedCallable, class_table::ClassTable, modules::CoreModuleLoader,
    proto_table::ProtoTable, AsyncJobQueue,
};

#[macro_export]
macro_rules! enqueue_job {
    ($state:expr, $future:expr) => {
        $state.job_queue().borrow().spawn(Box::pin($future));
    };
}

pub struct CoreState {
    job_queue: Arc<RefCell<AsyncJobQueue>>,
    module_loader: Rc<RefCell<CoreModuleLoader>>,
    timer_queue: Arc<TimerQueue<JsProctectedCallable>>,
    class_manager: Arc<ClassTable>,
    proto_manager: Arc<ProtoTable>,
}

impl Clone for CoreState {
    fn clone(&self) -> Self {
        CoreState {
            job_queue: self.job_queue.clone(),
            module_loader: self.module_loader.clone(),
            timer_queue: self.timer_queue.clone(),
            class_manager: self.class_manager.clone(),
            proto_manager: self.proto_manager.clone(),
        }
    }
}

impl CoreState {
    pub fn new(
        job_queue: AsyncJobQueue,
        timer_queue: TimerQueue<JsProctectedCallable>,
        manager: ClassTable,
        proto: ProtoTable,
        module_loader: CoreModuleLoader,
    ) -> CoreState {
        CoreState {
            job_queue: Arc::new(RefCell::new(job_queue)),
            module_loader: Rc::new(RefCell::new(module_loader)),
            timer_queue: Arc::new(timer_queue),
            class_manager: Arc::new(manager),
            proto_manager: Arc::new(proto),
        }
    }

    pub fn timers(&self) -> &TimerQueue<JsProctectedCallable> {
        &self.timer_queue
    }

    pub fn job_queue(&self) -> &Arc<RefCell<AsyncJobQueue>> {
        &self.job_queue
    }

    pub fn classes(&self) -> &Arc<ClassTable> {
        &self.class_manager
    }

    pub fn protos(&self) -> &Arc<ProtoTable> {
        &self.proto_manager
    }

    pub fn module_loader(&self) -> &Rc<RefCell<CoreModuleLoader>> {
        &self.module_loader
    }
}

pub fn downcast_state(
    context: &rust_jsc::JSContext,
) -> ManuallyDropClone<Box<CoreState>> {
    let state = context
        .get_shared_data::<CoreState>()
        .expect("state not found");
    ManuallyDropClone::new(state)
}
