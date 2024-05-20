use std::{mem::ManuallyDrop, ops::Deref};

use rust_jsc::JSContext;

use crate::{job::AsyncJobQueue, ManuallyDropClone, RuntimeState};

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
        let state = self
            .0
            .get_shared_data::<RuntimeState<AsyncJobQueue>>()
            .unwrap();
        ManuallyDropClone(ManuallyDrop::new(state))
    }

    pub fn set_state(&self, state: RuntimeState<AsyncJobQueue>) {
        self.0.set_shared_data(Box::new(state));
    }
}
