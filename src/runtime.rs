use std::task::{Context, Poll};

use futures::future::poll_fn;
use rust_jsc::{JSContext, JSObject, JSResult, JSValue};

use crate::{
    class_table::ClassTable,
    console::Console,
    context::KedoContext,
    file::FileSystem,
    file_dir::DirEntry,
    http::headers::{Headers, HeadersIterator},
    iterator::JsIterator,
    job::{AsyncJobQueue, JobQueue},
    proto_table::ProtoTable,
    timer_queue::{TimerJsCallable, TimerQueue},
    timers::Timer,
    RuntimeState,
};

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
        let mut class_table = ClassTable::new();
        let mut proto_table = ProtoTable::new();

        Self::init_class(&mut class_table, &context);
        Self::init_proto(&mut proto_table, &mut class_table, &context);

        let state = RuntimeState::new(job_queue, timer_queue, class_table, proto_table);
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

    fn init_class(class_manager: &mut ClassTable, ctx: &JSContext) {
        let global = ctx.global_object();
        DirEntry::init(class_manager, &ctx, &global).unwrap();
        Headers::init(class_manager, &ctx, &global).unwrap();
        JsIterator::init(class_manager).unwrap();
    }

    fn init_proto(
        proto_table: &mut ProtoTable,
        class_table: &mut ClassTable,
        ctx: &JSContext,
    ) {
        HeadersIterator::init_proto(proto_table, class_table, ctx)
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
            // TOOD: handle error
            if let Err(error) = callback.callable.call(None, args) {
                eprintln!("Error calling timer callback: {}", error.message().unwrap());
            }
        }
    }

    fn run_event_loop(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let callbaks = self.state.timers().poll_timers(cx);
        match callbaks {
            Poll::Ready(callbaks) => {
                self.call_callbaks(callbaks);
            }
            Poll::Pending => {}
        }

        let _ = self.state.job_queue().borrow_mut().poll(cx);
        self.state.job_queue().borrow().run_jobs(&self.context);
        if self.state.job_queue().borrow().is_empty() && self.state.timers().is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn garbage_collect(&self) {
        self.context.garbage_collect();
    }

    pub fn get_memory_usage(&self) -> JSObject {
        self.context.get_memory_usage()
    }

    pub async fn idle(&mut self) {
        poll_fn(|cx| self.run_event_loop(cx)).await;
    }
}
