use std::{
    cell::RefCell,
    collections::VecDeque,
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{stream::FuturesUnordered, StreamExt as _};
use rust_jsc::JSError;

pub type JsResult<T> = Result<T, JSError>;

pub struct NativeJob {
    f: Box<dyn FnOnce(&rust_jsc::JSContext) -> JsResult<()>>,
}

impl Debug for NativeJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeJob").field("f", &"Closure").finish()
    }
}

impl NativeJob {
    /// Creates a new `NativeJob` from a closure.
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(&rust_jsc::JSContext) -> JsResult<()> + 'static,
    {
        Self { f: Box::new(f) }
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
    /// Of all the requirements, Boa guarantees the first two by its internal implementation of `NativeJob`, meaning
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
}

pub struct AsyncJobQueue {
    jobs: RefCell<VecDeque<NativeJob>>,
    futures: FuturesUnordered<FutureJob>,
    waker: RefCell<Option<Waker>>,
}

impl Debug for AsyncJobQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AsyncQueue").field(&"..").finish()
    }
}

impl AsyncJobQueue {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            jobs: RefCell::new(VecDeque::new()),
            futures: FuturesUnordered::new(),
            waker: RefCell::new(None),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.borrow().is_empty() && self.futures.is_empty()
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        *self.waker.borrow_mut() = Some(cx.waker().clone());
        // Poll futures
        loop {
            match self.futures.poll_next_unpin(cx) {
                Poll::Ready(Some(job)) => {
                    self.enqueue_promise_job(job);
                }
                Poll::Ready(None) => break,
                Poll::Pending => break,
            }
        }

        // Check if there are more jobs
        if self.jobs.borrow().is_empty() && self.futures.is_empty() {
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

    pub fn spawn(&self, future: FutureJob) {
        self.futures.push(future);
        self.wake();
    }
}

impl JobQueue for AsyncJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob) {
        self.jobs.borrow_mut().push_back(job);
    }

    fn run_jobs(&self, context: &rust_jsc::JSContext) {
        let mut next_job = self.jobs.borrow_mut().pop_front();
        while let Some(job) = next_job {
            if job.call(context).is_err() {
                // TODO: Handle error
                // return;
            };
            next_job = self.jobs.borrow_mut().pop_front();
        }
    }

    fn enqueue_future_job(&self, future: FutureJob) {
        self.spawn(future);
    }
}
