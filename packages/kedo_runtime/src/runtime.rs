use futures::future::poll_fn;
use kedo_console::Console;
use kedo_core::{
    AsyncJobQueue, ClassTable, CoreModuleLoader, CoreState, JobQueue, ProtoTable,
};
use kedo_fs::FileSystemModuleLoader;
use kedo_std::TimerQueue;
use kedo_timers::Timer;
use kedo_utils::JSGlobalObject;
use kedo_web::{
    DecodedStreamResource, EncodingTextDecoder, FetchClientResource,
    FetchRequestResource, HttpRequestResource, InternalSignal,
    NetworkBufferChannelReaderResource, ReadableStreamResource,
    ReadableStreamResourceReader, RequestEventResource, UnboundedReadableStreamResource,
    UnboundedReadableStreamResourceReader, UrlRecord, WebModule,
};
use rust_jsc::{
    callback, uncaught_exception, uncaught_exception_event_loop, JSContext, JSError,
    JSFunction, JSObject, JSResult, JSString, JSValue,
};
use std::{
    sync::Arc,
    task::{Context, Poll},
};

pub struct Runtime {
    context: Arc<JSContext>,
    state: CoreState,
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.context.set_shared_data(Box::new(()));
    }
}

impl Runtime {
    pub fn new() -> Self {
        let context = JSContext::new();
        let timer_queue = TimerQueue::new();
        let job_queue = AsyncJobQueue::new();
        let mut class_table = ClassTable::new();
        let mut proto_table = ProtoTable::new();
        let mut module_loader = CoreModuleLoader::default();

        Self::init_class(&mut class_table);
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

        let state = CoreState::new(
            job_queue,
            timer_queue,
            class_table,
            proto_table,
            module_loader,
        );
        context.set_shared_data(Box::new(state.clone()));
        let runtime = Runtime {
            context: Arc::new(context),
            state,
        };

        runtime.init_module();
        runtime
    }

    pub fn add_loader(&self, loader: impl kedo_core::ModuleLoader + 'static) {
        let mut module_loader = self.state.module_loader().borrow_mut();
        module_loader.add_loader(loader);
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
        let error = JSError::from(args[0].as_object()?);
        println!(
            "Error unhandled: {} - {}",
            args[1].as_string().unwrap(),
            error.stack()?,
            // args.len()
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

    fn init_module_loaders(module_loader: &mut CoreModuleLoader) {
        module_loader.add_source(WebModule);
        module_loader.add_source(FileSystemModuleLoader);
    }

    fn init_module(&self) {
        self.state.module_loader().borrow().init(&self.context);
        Console::init_globals(&self.context).unwrap();
        Timer::init_globals(&self.context).unwrap();
        let kedo = JSObject::new(&self.context);
        kedo.protect();

        self.context
            .global_object()
            .set_property("Kedo", &kedo, Default::default())
            .unwrap();
    }

    fn init_class(class_manager: &mut ClassTable) {
        UrlRecord::init_class(class_manager).expect("Failed to init UrlRecord");
        EncodingTextDecoder::init_class(class_manager)
            .expect("Failed to init EncodingTextDecoder");
        ReadableStreamResourceReader::init_class(class_manager)
            .expect("Failed to init ReadableStreamResourceReader");
        UnboundedReadableStreamResourceReader::init_class(class_manager)
            .expect("Failed to init UnboundedReadableStreamResourceReader");
        FetchClientResource::init_class(class_manager)
            .expect("Failed to init FetchClientResource");
        InternalSignal::init_class(class_manager).expect("Failed to init InternalSignal");
        FetchRequestResource::init_class(class_manager)
            .expect("Failed to init FetchRequestResource");
        HttpRequestResource::init_class(class_manager)
            .expect("Failed to init HttpRequestResource");
        ReadableStreamResource::init_class(class_manager)
            .expect("Failed to init JsReadableStream");
        UnboundedReadableStreamResource::init_class(class_manager)
            .expect("Failed to init JsUnboundedReadableStream");
        DecodedStreamResource::init_class(class_manager)
            .expect("Failed to init JsResponseBody");
        RequestEventResource::init_class(class_manager)
            .expect("Failed to init RequestEventResource");
        NetworkBufferChannelReaderResource::init_class(class_manager)
            .expect("Failed to init NetworkBufferChannelReaderResource");
    }

    fn init_proto(
        proto_table: &mut ProtoTable,
        class_table: &mut ClassTable,
        ctx: &JSContext,
    ) {
        UrlRecord::init_proto(proto_table, class_table, ctx).unwrap();
        ReadableStreamResource::init_proto(proto_table, class_table, ctx).unwrap();
        UnboundedReadableStreamResource::init_proto(proto_table, class_table, ctx)
            .unwrap();
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

    fn run_event_loop(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let callbaks = self.state.timers().poll_timers(cx);
        if let Poll::Ready(callbaks) = callbaks {
            for callback in callbaks {
                if let Err(error) = callback.call() {
                    println!("Error timer callback: {}", error.message().unwrap());
                }
            }
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
