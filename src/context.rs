use std::{mem::ManuallyDrop, ops::Deref};

use rust_jsc::JSContext;

use crate::{job::{AsyncJobQueue, JobQueue}, ManuallyDropClone, RuntimeState};

pub struct KedoContext<'js>(&'js JSContext);

impl Deref for KedoContext<'_> {
    type Target = JSContext;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'js> KedoContext<'js> {
    pub fn from(context: &'js JSContext) -> Self {
        KedoContext(context)
    }

    pub fn state(&self) -> ManuallyDropClone<Box<RuntimeState<AsyncJobQueue>>> {
        downcast_state(self)
    }

    pub fn set_state(&self, state: RuntimeState<AsyncJobQueue>) {
        self.0.set_shared_data(Box::new(state));
    }
}

pub fn downcast_state<T: JobQueue>(context: &JSContext) -> ManuallyDropClone<Box<RuntimeState<T>>> {
    let state = context.get_shared_data::<RuntimeState<T>>().unwrap();
    ManuallyDropClone(ManuallyDrop::new(state))
}