use std::task::{Context, Poll};

use futures::future::poll_fn;
use rust_jsc::{
    callback, uncaught_exception, uncaught_exception_event_loop, JSContext, JSFunction,
    JSObject, JSResult, JSString, JSValue,
};

use crate::{
    class_table::ClassTable,
    console::Console,
    context::KedoContext,
    file::FileSystem,
    file_dir::DirEntry,
    http::{headers::HeadersIterator, response::JSResponseBodyStreamResource},
    iterator::JsIterator,
    job::{AsyncJobQueue, JobQueue},
    module::KedoModuleLoader,
    modules::{self, text_decoder_inner::EncodingTextDecoder, url_record::UrlRecord},
    promise::InternalPromise,
    proto_table::ProtoTable,
    signals::InternalSignal,
    std_modules,
    streams::streams::JSReadableStreamResource,
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
        let mut module_loader = KedoModuleLoader::default();

        Self::init_class(&mut class_table, &context);
        Self::init_proto(&mut proto_table, &mut class_table, &context);
        Self::init_module_loaders(&mut module_loader);

        let unhandled_rejection = JSFunction::callback(
            &context,
            Some("unhandled_rejection"),
            Some(Self::unhandled_rejection),
        );
        unhandled_rejection.protect(); // Protect the function from GC
        context
            .set_unhandled_rejection_callback(unhandled_rejection.into())
            .unwrap();
        context.set_uncaught_exception_handler(Some(Self::uncaught_exception));
        context.set_uncaught_exception_at_event_loop_callback(Some(
            Self::uncaught_exception_event_loop,
        ));

        let state = RuntimeState::new(
            job_queue,
            timer_queue,
            class_table,
            proto_table,
            module_loader,
        );
        kedo_context.set_state(state.clone());
        let runtime = Runtime { context, state };

        runtime.init_module();
        runtime
    }

    pub fn evaluate_module(&self, filename: &str) -> JSResult<()> {
        self.context.evaluate_module(filename)
    }

    #[callback]
    fn unhandled_rejection(
        ctx: JSContext,
        _function: JSObject,
        _this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        println!(
            "Error unhandled: {}, {}",
            args[1].as_string().unwrap(),
            args.len()
        );
        Ok(JSValue::undefined(&ctx))
    }

    #[uncaught_exception]
    fn uncaught_exception(_ctx: JSContext, _filename: JSString, exception: JSValue) {
        println!("Uncaught exception: {:?}", exception.as_string().unwrap());
    }

    #[uncaught_exception_event_loop]
    fn uncaught_exception_event_loop(_ctx: JSContext, exception: JSValue) {
        println!(
            "Uncaught exception in event loop: {:?}",
            exception.as_string().unwrap()
        );
    }

    pub fn evaluate_module_from_source(
        &self,
        source: &str,
        source_url: &str,
        starting_line_number: Option<i32>,
    ) -> JSResult<()> {
        self.context
            .evaluate_module_from_source(source, source_url, starting_line_number)
    }

    fn init_module_loaders(module_loader: &mut KedoModuleLoader) {
        module_loader.register_loader(std_modules::StdModuleLoader);
        module_loader.register_resolver(std_modules::StdModuleResolver);
        module_loader.register_resolver(modules::InternalModuleResolver);
        module_loader.register_synthetic_module(modules::utils::UtilsModule);
    }

    fn init_module(&self) {
        self.state.module_loader().init(&self.context);
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
        DirEntry::init(class_manager, &ctx, &global).expect("Failed to init DirEntry");
        UrlRecord::init_class(class_manager).expect("Failed to init UrlRecord");
        EncodingTextDecoder::init_class(class_manager)
            .expect("Failed to init EncodingTextDecoder");
        JsIterator::init(class_manager).expect("Failed to init JsIterator");
        JSReadableStreamResource::init_class(class_manager)
            .expect("Failed to init JsReadableStream");
        InternalPromise::init_class(class_manager)
            .expect("Failed to init InternalPromise");
        InternalSignal::init_class(class_manager).expect("Failed to init InternalSignal");
        JSReadableStreamResource::init_class(class_manager)
            .expect("Failed to init JsReadableStream");
        JSResponseBodyStreamResource::init_class(class_manager)
            .expect("Failed to init JsResponseBody");
    }

    fn init_proto(
        proto_table: &mut ProtoTable,
        class_table: &mut ClassTable,
        ctx: &JSContext,
    ) {
        HeadersIterator::init_proto(proto_table, class_table, ctx);
        UrlRecord::init_proto(proto_table, class_table, ctx).unwrap();
        JSReadableStreamResource::init_proto(proto_table, class_table, ctx).unwrap();
        EncodingTextDecoder::init_proto(proto_table, class_table, ctx).unwrap();
        InternalSignal::init_proto(proto_table, class_table, ctx).unwrap();
    }

    pub fn context(&self) -> &JSContext {
        &self.context
    }

    pub fn evaluate_script(&self, script: &str, line: Option<i32>) -> JSResult<JSValue> {
        self.context.evaluate_script(script, line)
    }

    pub fn link_and_evaluate(&self, key: &str) -> JSValue {
        self.context.link_and_evaluate_module(key)
    }

    pub fn check_syntax(&self, script: &str, line: Option<i32>) -> JSResult<bool> {
        self.context.check_syntax(script, line.unwrap_or(0))
    }

    fn call_callbaks(&self, callbaks: Vec<TimerJsCallable>) {
        for callback in callbaks {
            let args: &[JSValue] = callback.args.as_slice();
            // TOOD: handle error
            if let Err(error) = callback.callable.call(None, args) {
                println!("Error calling timer callback: {}", error.message().unwrap());
            }
        }
    }

    fn run_event_loop(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let callbaks = self.state.timers().poll_timers(cx);
        if let Poll::Ready(callbaks) = callbaks {
            self.call_callbaks(callbaks);
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
