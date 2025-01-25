use std::{
    cell::RefCell,
    collections::VecDeque,
    fmt::Debug,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
};

use futures::{stream::FuturesUnordered, StreamExt as _};
use rust_jsc::JSError;

pub type JsResult<T> = Result<T, JSError>;

#[macro_export]
macro_rules! native_job {
    ($tag:expr, $closure:expr) => {{
        let job = kedo_core::NativeJob::new($closure);
        #[cfg(debug_assertions)]
        let job = job.set_tag($tag);
        job
    }};
}

pub struct NativeJob {
    f: Box<dyn FnOnce(&rust_jsc::JSContext) -> JsResult<()>>,
    // add attribute to job name for debugging only
    #[cfg(debug_assertions)]
    tag: String,
}

impl Debug for NativeJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("NativeJob");
        debug_struct.field("f", &"Closure");
        #[cfg(debug_assertions)]
        debug_struct.field("tag", &self.tag);
        debug_struct.finish()
    }
}

impl NativeJob {
    /// Creates a new `NativeJob` from a closure.
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(&rust_jsc::JSContext) -> JsResult<()> + 'static,
    {
        Self {
            f: Box::new(f),
            #[cfg(debug_assertions)]
            tag: "Unnamed".to_string(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn set_tag(mut self, name: &str) -> Self {
        self.tag = name.to_string();
        self
    }

    /// Calls the native job with the specified [`Context`].
    ///
    /// # Note
    ///
    /// If the native job has an execution realm defined, this sets the running execution
    /// context to the realm's before calling the inner closure, and resets it after execution.
    pub fn call(self, context: &rust_jsc::JSContext) -> JsResult<()> {
        (self.f)(context)
    }
}

pub type FutureJob = Pin<Box<dyn Future<Output = NativeJob> + 'static>>;

/// A queue of `ECMAscript` [Jobs].
///
/// This is the main API that allows creating custom event loops with custom job queues.
///
/// [Jobs]: https://tc39.es/ecma262/#sec-jobs
pub trait JobQueue {
    /// [`HostEnqueuePromiseJob ( job, realm )`][spec].
    ///
    /// Enqueues a [`NativeJob`] on the job queue.
    ///
    /// # Requirements
    ///
    /// Per the [spec]:
    /// > An implementation of `HostEnqueuePromiseJob` must conform to the requirements in [9.5][Jobs] as well as the
    ///   following:
    /// > - If `realm` is not null, each time `job` is invoked the implementation must perform implementation-defined steps
    ///     such that execution is prepared to evaluate ECMAScript code at the time of job's invocation.
    /// > - Let `scriptOrModule` be `GetActiveScriptOrModule()` at the time `HostEnqueuePromiseJob` is invoked. If realm
    ///     is not null, each time job is invoked the implementation must perform implementation-defined steps such that
    ///     `scriptOrModule` is the active script or module at the time of job's invocation.
    /// > - Jobs must run in the same order as the `HostEnqueuePromiseJob` invocations that scheduled them.
    ///
    /// the implementer must only guarantee that jobs are run in the same order as they're enqueued.
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-hostenqueuepromisejob
    /// [Jobs]: https://tc39.es/ecma262/#sec-jobs
    fn enqueue_promise_job(&self, job: NativeJob);

    /// Runs all jobs in the queue.
    ///
    /// Running a job could enqueue more jobs in the queue. The implementor of the trait
    /// determines if the method should loop until there are no more queued jobs or if
    /// it should only run one iteration of the queue.
    fn run_jobs(&self, context: &rust_jsc::JSContext);

    /// Enqueues a new [`Future`] job on the job queue.
    ///
    /// On completion, `future` returns a new [`NativeJob`] that needs to be enqueued into the
    /// job queue to update the state of the inner `Promise`, which is what ECMAScript sees. Failing
    /// to do this will leave the inner `Promise` in the `pending` state, which won't call any `then`
    /// or `catch` handlers, even if `future` was already completed.
    fn enqueue_future_job(&self, future: FutureJob);

    fn len(&self) -> usize;
}

/// A simple FIFO job queue that bails on the first error.
///
/// This is the default job queue for the [`Context`], but it is mostly pretty limited for
/// custom event queues.
///
/// To disable running promise jobs on the engine, see [`IdleJobQueue`].
#[derive(Default)]
pub struct SimpleJobQueue {
    jobs: RefCell<VecDeque<NativeJob>>,
}

impl Debug for SimpleJobQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SimpleQueue").field(&"..").finish()
    }
}

impl SimpleJobQueue {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobQueue for SimpleJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob) {
        self.jobs.borrow_mut().push_back(job);
    }

    fn run_jobs(&self, context: &rust_jsc::JSContext) {
        // Yeah, I have no idea why Rust extends the lifetime of a `RefCell` that should be immediately
        // dropped after calling `pop_front`.
        let mut next_job = self.jobs.borrow_mut().pop_front();
        while let Some(job) = next_job {
            if job.call(context).is_err() {
                // TODO: Handle error
                // return;
            };
            next_job = self.jobs.borrow_mut().pop_front();
        }
    }

    fn enqueue_future_job(&self, _: FutureJob) {
        panic!("Future jobs are not supported in the simple job queue");
    }

    fn len(&self) -> usize {
        self.jobs.borrow().len()
    }
}

pub struct FutureJobWrapper {
    future: FutureJob,
    // If true, the future prevents the event loop from exiting
    prevent_exit: bool,
    queue: Weak<RefCell<AsyncJobQueueInner>>,
}

impl Future for FutureJobWrapper {
    type Output = NativeJob;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = Pin::new(&mut self.future);
        let poll_result = future.poll(cx);
        if let Poll::Ready(_) = &poll_result {
            if self.prevent_exit {
                if let Some(queue) = self.queue.upgrade() {
                    queue.borrow_mut().futures_prevent_exit_count -= 1;
                }
            }
        }
        poll_result
    }
}

pub struct AsyncJobQueueInner {
    pub jobs: VecDeque<NativeJob>,
    // Count of futures that prevent the event loop from exiting
    pub futures_prevent_exit_count: usize,
}

impl AsyncJobQueueInner {
    pub fn push_job(&mut self, job: NativeJob) {
        self.jobs.push_back(job);
    }
}

pub struct AsyncJobQueue {
    inner: Rc<RefCell<AsyncJobQueueInner>>,
    futures: FuturesUnordered<FutureJobWrapper>,
    waker: RefCell<Option<Waker>>,
}

impl Debug for AsyncJobQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AsyncQueue").field(&"..").finish()
    }
}

impl AsyncJobQueue {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(AsyncJobQueueInner {
                jobs: VecDeque::new(),
                futures_prevent_exit_count: 0,
            })),
            futures: FuturesUnordered::new(),
            waker: RefCell::new(None),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().jobs.is_empty()
            && self.inner.borrow().futures_prevent_exit_count == 0
    }

    pub fn leak(&self) -> Weak<RefCell<AsyncJobQueueInner>> {
        Rc::downgrade(&self.inner)
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        *self.waker.borrow_mut() = Some(cx.waker().clone());
        while let Poll::Ready(Some(job)) = self.futures.poll_next_unpin(cx) {
            self.enqueue_promise_job(job);
        }

        if self.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn wake(&self) {
        if let Some(waker) = self.waker.borrow().as_ref() {
            waker.wake_by_ref();
        }
    }

    /// Spawns a new future job on the queue.
    /// This method is used to spawn a new future job on the queue.
    /// that does not prevent the event loop from exiting.
    pub fn spawn_non_blocking(&self, future: FutureJob) {
        let wrapper = FutureJobWrapper {
            future,
            prevent_exit: false,
            queue: Weak::new(),
        };
        self.futures.push(wrapper);
    }

    /// Spawns a new future job on the queue.
    /// This method is used to spawn a new future job on the queue.
    /// that prevents the event loop from exiting.
    pub fn spawn(&self, future: FutureJob) {
        let wrapper = FutureJobWrapper {
            future,
            prevent_exit: true,
            queue: Rc::downgrade(&self.inner),
        };
        self.inner.borrow_mut().futures_prevent_exit_count += 1;
        self.futures.push(wrapper);
        self.wake();
    }
}

impl JobQueue for AsyncJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob) {
        self.inner.borrow_mut().jobs.push_back(job);
    }

    fn run_jobs(&self, context: &rust_jsc::JSContext) {
        let mut next_job = self.inner.borrow_mut().jobs.pop_front();
        while let Some(job) = next_job {
            #[cfg(debug_assertions)]
            let tag = job.tag.clone();

            let result = job.call(context);
            if result.is_err() {
                #[cfg(debug_assertions)]
                {
                    let error = result.unwrap_err();
                    println!("Job: {:?}, Error: {:?}", tag, error.message().unwrap());
                    println!("Stack: {:?}", error.stack().unwrap());
                }

                // TODO: Handle error
            };
            next_job = self.inner.borrow_mut().jobs.pop_front();
        }
    }

    fn enqueue_future_job(&self, future: FutureJob) {
        self.spawn(future);
    }

    fn len(&self) -> usize {
        self.inner.borrow().jobs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::{Rc, Weak};
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    // Helper function to create a no-op waker
    fn noop_waker() -> Waker {
        fn clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}
        static VTABLE: RawWakerVTable =
            RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
    }

    #[test]
    fn test_enqueue_promise_job() {
        // Test that a NativeJob can be enqueued and executed
        let context = rust_jsc::JSContext::new();
        let queue = AsyncJobQueue::new();
        let executed = Rc::new(RefCell::new(false));

        let job = NativeJob::new({
            let executed = executed.clone();
            move |_ctx| {
                *executed.borrow_mut() = true;
                Ok(())
            }
        });

        queue.enqueue_promise_job(job);
        queue.run_jobs(&context);

        assert_eq!(*executed.borrow(), true);
    }

    #[test]
    fn test_enqueue_future_job() {
        // Test that a FutureJob can be enqueued and executed
        let context = rust_jsc::JSContext::new();
        let mut queue = AsyncJobQueue::new();
        let executed = Rc::new(RefCell::new(false));

        let executed_clone = executed.clone();
        let future_job: FutureJob = Box::pin(async {
            NativeJob::new({
                move |_ctx| {
                    *executed_clone.borrow_mut() = true;
                    Ok(())
                }
            })
        });

        queue.enqueue_future_job(future_job);

        // Poll the queue to progress the future
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        while queue.poll(&mut cx).is_pending() {
            queue.run_jobs(&context);
        }

        assert_eq!(*executed.borrow(), true);
    }

    #[test]
    fn test_multiple_jobs_execution_order() {
        // Test that jobs are executed in the order they are enqueued
        let context = rust_jsc::JSContext::new();
        let queue = AsyncJobQueue::new();
        let execution_order = Rc::new(RefCell::new(vec![]));

        for i in 0..5 {
            let order = execution_order.clone();
            let job = NativeJob::new(move |_ctx| {
                order.borrow_mut().push(i);
                Ok(())
            });
            queue.enqueue_promise_job(job);
        }

        queue.run_jobs(&context);

        assert_eq!(*execution_order.borrow(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_future_job_wrapper_poll() {
        // Test that FutureJobWrapper correctly polls the inner future
        let polled = Rc::new(RefCell::new(false));

        let future = TestFuture {
            polled: polled.clone(),
        };

        let mut wrapper = FutureJobWrapper {
            future: Box::pin(future),
            prevent_exit: false,
            queue: Weak::new(),
        };

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        let poll_result = wrapper.future.as_mut().poll(&mut cx);

        assert_eq!(*polled.borrow(), true);
        assert!(poll_result.is_ready());
    }

    struct TestFuture {
        polled: Rc<RefCell<bool>>,
    }

    impl Future for TestFuture {
        type Output = NativeJob;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            *self.polled.borrow_mut() = true;
            Poll::Ready(NativeJob::new(|_ctx| Ok(())))
        }
    }

    #[test]
    fn test_async_job_queue_waker() {
        // Test that the waker is stored and can wake the executor
        let mut queue = AsyncJobQueue::new();

        // Initially, waker should be None
        assert!(queue.waker.borrow().is_none());

        // Create a waker and poll the queue
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let _ = queue.poll(&mut cx);

        // Waker should now be set
        assert!(queue.waker.borrow().is_some());
    }

    #[test]
    fn test_is_empty() {
        // Test that is_empty reflects the correct state
        let queue = AsyncJobQueue::new();

        assert!(queue.is_empty());

        queue.enqueue_promise_job(NativeJob::new(|_ctx| Ok(())));

        assert!(!queue.is_empty());
    }

    #[test]
    fn test_prevent_exit() {
        // Test that futures with prevent_exit set keep the queue from being empty
        let mut queue = AsyncJobQueue::new();
        let context = rust_jsc::JSContext::new();

        // Enqueue a future job that prevents exit
        let future_job: FutureJob = Box::pin(async { NativeJob::new(|_ctx| Ok(())) });
        queue.enqueue_future_job(future_job);

        assert!(!queue.is_empty());

        // Poll the queue to progress the future
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let _ = queue.poll(&mut cx);

        assert!(!queue.is_empty());
        queue.run_jobs(&context);

        // Since the future completes immediately, the queue should now be empty
        assert!(queue.is_empty());
    }

    #[test]
    fn test_spawn_non_blocking() {
        // Test that spawn_non_blocking does not prevent exit
        let queue = AsyncJobQueue::new();

        // Enqueue a non-blocking future job
        let future_job: FutureJob = Box::pin(async { NativeJob::new(|_ctx| Ok(())) });
        queue.spawn_non_blocking(future_job);

        assert!(queue.is_empty());
    }
}
