use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use class_table::ClassTable;
use job::JobQueue;
use proto_table::ProtoTable;
use timer_queue::TimerQueue;

mod async_util;
mod class_table;
mod console;
mod context;
mod errors;
mod file;
mod file_dir;
mod http;
mod iterator;
mod job;
mod proto_table;
mod timer_queue;
mod timers;
mod utils;

pub mod runtime;

pub(crate) struct ManuallyDropArc<T>(ManuallyDrop<Arc<T>>);

impl<T> ManuallyDropArc<T> {
    #[allow(unused)]
    pub fn clone(&self) -> Arc<T> {
        self.0.deref().clone()
    }
}

impl<T> Deref for ManuallyDropArc<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for ManuallyDropArc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

pub(crate) struct ManuallyDropClone<T>(ManuallyDrop<T>);

impl<T> ManuallyDropClone<T> {
    #[allow(unused)]
    pub fn clone(&self) -> T
    where
        T: Clone,
    {
        self.0.deref().clone()
    }

    pub fn take(self) -> T {
        ManuallyDrop::into_inner(self.0)
    }
}

impl<T> Deref for ManuallyDropClone<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for ManuallyDropClone<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

pub(crate) struct ManuallyDropRc<T>(ManuallyDrop<Rc<T>>);

impl<T> ManuallyDropRc<T> {
    #[allow(unused)]
    pub fn clone(&self) -> Rc<T> {
        self.0.deref().clone()
    }
}

impl<T> Deref for ManuallyDropRc<T> {
    type Target = Rc<T>;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for ManuallyDropRc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

pub struct RuntimeState<T>
where
    T: JobQueue,
{
    job_queue: Arc<RefCell<T>>,
    timer_queue: Arc<TimerQueue>,
    class_manager: Arc<ClassTable>,
    proto_manager: Arc<ProtoTable>,
}

impl<T> Clone for RuntimeState<T>
where
    T: JobQueue,
{
    fn clone(&self) -> Self {
        RuntimeState {
            job_queue: self.job_queue.clone(),
            timer_queue: self.timer_queue.clone(),
            class_manager: self.class_manager.clone(),
            proto_manager: self.proto_manager.clone(),
        }
    }
}

impl<T> RuntimeState<T>
where
    T: JobQueue,
{
    pub fn new(
        job_queue: T,
        timer_queue: TimerQueue,
        manager: ClassTable,
        proto: ProtoTable,
    ) -> RuntimeState<T> {
        RuntimeState {
            job_queue: Arc::new(RefCell::new(job_queue)),
            timer_queue: Arc::new(timer_queue),
            class_manager: Arc::new(manager),
            proto_manager: Arc::new(proto),
        }
    }

    pub fn timers(&self) -> &TimerQueue {
        &self.timer_queue
    }

    pub fn job_queue(&self) -> &Arc<RefCell<T>> {
        &self.job_queue
    }

    pub fn classes(&self) -> &Arc<ClassTable> {
        &self.class_manager
    }

    pub fn protos(&self) -> &Arc<ProtoTable> {
        &self.proto_manager
    }
}
