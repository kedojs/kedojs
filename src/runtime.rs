use std::{
    cell::RefCell,
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::poll_fn;
use rust_jsc::{JSContext, JSResult, JSValue};

use crate::{
    class_manager::ClassManager,
    console::Console,
    context::KedoContext,
    file::FileSystem,
    file_dir::FsDirEntry,
    job::{AsyncJobQueue, JobQueue},
    timer_queue::{TimerJsCallable, TimerQueue},
    timers::Timer,
    RuntimeState,
};

impl<T> Clone for RuntimeState<T>
where
    T: JobQueue,
{
    fn clone(&self) -> Self {
        RuntimeState {
            job_queue: self.job_queue.clone(),
            timer_queue: self.timer_queue.clone(),
            class_manager: self.class_manager.clone(),
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
        manager: ClassManager,
    ) -> RuntimeState<T> {
        RuntimeState {
            job_queue: Arc::new(RefCell::new(job_queue)),
            timer_queue: Arc::new(timer_queue),
            class_manager: Arc::new(manager),
        }
    }
}

pub struct Runtime {
    context: JSContext,
    state: RuntimeState<AsyncJobQueue>,
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.context.set_shared_data(Box::new(()));
    }
}

impl Runtime {
    pub fn new() -> Self {
        let context = JSContext::new();
        let kedo_context = KedoContext::from(&context);
        let timer_queue = TimerQueue::new();
        let job_queue = AsyncJobQueue::new();
        let mut class_manager = ClassManager::new();

        Self::init_class(&mut class_manager, &context);
        let state = RuntimeState::new(job_queue, timer_queue, class_manager);
        kedo_context.set_state(state.clone());
        let runtime = Runtime { context, state };

        runtime.init_module();
        runtime
    }

    pub fn evaluate_module(&self, filename: &str) -> JSResult<()> {
        self.context.evaluate_module(filename)
    }

    fn init_module(&self) {
        Console::init(&self.context).unwrap();
        Timer::init(&self.context).unwrap();
        let fs = FileSystem::object(&self.context).unwrap();
        self.context
            .global_object()
            .set_property("Kedo", &fs, Default::default())
            .unwrap();
    }

    fn init_class(class_manager: &mut ClassManager, ctx: &JSContext) {
        FsDirEntry::init(class_manager).unwrap();

        class_manager.register(&ctx);
    }

    pub fn evaluate_script(&self, script: &str, line: Option<i32>) -> JSResult<JSValue> {
        self.context.evaluate_script(script, line)
    }

    pub fn check_syntax(&self, script: &str, line: Option<i32>) -> JSResult<bool> {
        self.context.check_syntax(script, line.unwrap_or(0))
    }

    fn call_callbaks(&self, callbaks: Vec<TimerJsCallable>) {
        for callback in callbaks {
            let args: &[JSValue] = callback.args.as_slice();
            callback.callable.call(None, args).unwrap();
        }
    }

    fn run_event_loop(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let callbaks = self.state.timer_queue.poll_timers(cx);
        match callbaks {
            Poll::Ready(callbaks) => {
                self.call_callbaks(callbaks);
            }
            Poll::Pending => {}
        }

        let _ = self.state.job_queue.borrow_mut().poll(cx);
        self.state.job_queue.borrow().run_jobs(&self.context);
        if self.state.job_queue.borrow().is_empty() && self.state.timer_queue.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub async fn idle(&mut self) {
        poll_fn(|cx| self.run_event_loop(cx)).await;
    }
}
