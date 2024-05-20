use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use class_manager::ClassManager;
use job::JobQueue;
use timer_queue::TimerQueue;

mod async_util;
mod console;
mod context;
mod errors;
mod file;
mod file_dir;
mod job;
mod timer_queue;
mod timers;
mod class_manager;

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
    pub job_queue: Arc<RefCell<T>>,
    pub timer_queue: Arc<TimerQueue>,
    pub class_manager: Arc<ClassManager>,
}
