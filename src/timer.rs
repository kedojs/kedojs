use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use boa_engine::job::NativeJob;
use boa_engine::{js_string, JsResult, JsValue, NativeFunction};
use std::task::{Context, Poll, Waker};
use tokio::time::{sleep_until, Duration, Instant, Sleep};

#[derive(Clone)]
pub struct TimerJsCallable {
  pub callable: JsValue,
  pub args: Vec<JsValue>,
}

pub type TimerId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerType {
  Timeout,
  Interval,
}

// TODO - !("Add CompoundedTimerKey struct to execute tasks in order")
// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
// struct CompoundedTimerKey(Instant, TimerId);

#[derive(Clone)]
pub struct TimerData {
  id: TimerId,
  duration: Duration,
  timer_type: TimerType,
}

// create a mutable sleep struct
pub struct SleepWrapper {
  sleep: RefCell<Option<Sleep>>,
}

pub struct TimerQueue {
  timers: RefCell<BTreeMap<Instant, Vec<TimerData>>>,
  callback_map: RefCell<HashMap<TimerId, TimerJsCallable>>,
  next_timer_id: Cell<TimerId>,
  sleep: Box<SleepWrapper>,
  waker: RefCell<Option<Waker>>,
}

impl TimerQueue {
  pub fn new() -> Self {
    Self {
      timers: RefCell::new(BTreeMap::new()),
      callback_map: RefCell::new(HashMap::new()),
      next_timer_id: Cell::new(0),
      sleep: Box::new(SleepWrapper {
        sleep: RefCell::new(None),
      }),
      waker: RefCell::new(None),
    }
  }

  pub fn add_timer(
    &self,
    duration: Duration,
    timer_type: TimerType,
    callback: TimerJsCallable,
  ) -> TimerId {
    let expiration = Instant::now() + duration;
    let id = self.next_timer_id.get() + 1;
    self.next_timer_id.set(id);

    let mut timers = self.timers.borrow_mut();
    timers
      .entry(expiration)
      .or_insert_with(Vec::new)
      .push(TimerData {
        id,
        timer_type,
        duration,
      });
    self.callback_map.borrow_mut().insert(id, callback);
    self.update_sleep_timer(expiration);

    id
  }

  fn update_sleep_timer(&self, new_expiration: Instant) {
    let mut sleep = self.sleep.sleep.borrow_mut();
    let should_update = match &*sleep {
      Some(existing_sleep) => new_expiration < existing_sleep.deadline(),
      None => true,
    };

    if should_update {
      *sleep = Some(sleep_until(new_expiration));
      if let Some(waker) = self.waker.borrow().as_ref() {
        waker.wake_by_ref();
      }
    }
  }

  pub fn clear_timer(&self, id: &TimerId) {
    self.callback_map.borrow_mut().remove(id);
  }

  #[allow(dead_code)]
  pub fn is_empty(&self) -> bool {
    self.timers.borrow().is_empty()
  }

  fn enqueue_native_jobs(
    &self,
    context: &mut boa_engine::Context,
    tasks: Vec<TimerJsCallable>,
  ) {
    for task in tasks {
      self.enqueue_native_job(context, task);
    }
  }

  fn enqueue_native_job(&self, context: &mut boa_engine::Context, task: TimerJsCallable) {
    context.job_queue().enqueue_promise_job(
      NativeJob::new(move |context| {
        let callable = task.callable.as_callable().unwrap();

        Ok(
          callable
            .call(&JsValue::undefined(), &task.args, context)
            .unwrap(),
        )
      }),
      context,
    );
  }

  pub fn poll_timers(
    &self,
    cx: &mut Context<'_>,
    bcx: &mut boa_engine::Context,
  ) -> Poll<()> {
    let now = Instant::now();

    // 1. check for ready timers and remove them after calling their callbacks
    // 2. poll the sleep future to determine when the next timer will expire
    // 3. if all timers are expired, return Poll::Ready(()), otherwise return Poll::Pending
    let mut timers = self.timers.borrow_mut();
    let keys: Vec<_> = timers.keys().cloned().take_while(|&k| k <= now).collect();

    // hold tasks for deferred execution
    let mut tasks = Vec::new();

    for key in keys {
      // 1. remove timer and callbacks from the map if timer type is Timeout
      // check if the timer type is Timeout before removing it dont call the remove method
      // get the timer data from the timers map
      let data = timers.remove(&key).unwrap();

      for timer in data {
        // remove the callback from the callback map if the timer type is Timeout
        if timer.timer_type == TimerType::Timeout {
          if let Some(callback) = self.callback_map.borrow_mut().remove(&timer.id) {
            tasks.push(callback);
          }
        } else {
          // if the timer type is Interval, add the timer back to the timers map
          // with a new expiration time
          let expiration = Instant::now() + timer.duration;
          let timer_id = timer.id;
          if let Some(callback) = self.callback_map.borrow_mut().get(&timer_id) {
            timers
              .entry(expiration)
              .or_insert_with(Vec::new)
              .push(timer);
            tasks.push(callback.clone());
          }
        };
      }
    }

    if !timers.is_empty() {
      *self.waker.borrow_mut() = Some(cx.waker().clone());

      let next_expiration = timers.keys().next().unwrap();
      // queue tasks for deferred execution after dropping timers borrow
      self.enqueue_native_jobs(bcx, tasks);

      // new sleep from the next expiration
      *self.sleep.sleep.borrow_mut() = Some(sleep_until(*next_expiration));

      // TODO - Remove
      // println!(
      //   "Polling sleep {:?}",
      //   self.sleep.sleep.borrow_mut().as_mut().unwrap().deadline()
      // );

      let pin = unsafe {
        Pin::new_unchecked(&mut *self.sleep.sleep.borrow_mut().as_mut().unwrap())
          .poll(cx)
          .is_ready()
      };

      if pin {
        self.waker.borrow().as_ref().unwrap().wake_by_ref();
      }

      return Poll::Pending;
    }

    // queue tasks for deferred execution after dropping timers borrow
    self.enqueue_native_jobs(bcx, tasks);

    Poll::Ready(())
  }
}

pub struct Timer {
  timers: Rc<RefCell<TimerQueue>>,
}

// Implement the Timer struct
impl Timer {
  const SET_TIMEOUT_NAME: &'static str = "setTimeout";
  const SET_INTERVAL_NAME: &'static str = "setInterval";
  const CLEAR_TIMEOUT_NAME: &'static str = "clearTimeout";
  const CLEAR_INTERVAL_NAME: &'static str = "clearInterval";

  pub fn register_timers(
    context: &mut boa_engine::Context,
    timers: Rc<RefCell<TimerQueue>>,
  ) {
    let timer = Timer { timers };
    let state = Rc::new(RefCell::new(timer));

    context.register_global_builtin_callable(
      js_string!(Self::SET_TIMEOUT_NAME),
      0,
      Self::timer_function(Self::set_timeout, state.clone()),
    ).unwrap();

    context.register_global_builtin_callable(
      js_string!(Self::CLEAR_TIMEOUT_NAME),
      0,
      Self::timer_function(Self::clear_timer, state.clone()),
    ).unwrap();

    context.register_global_builtin_callable(
      js_string!(Self::CLEAR_INTERVAL_NAME),
      0,
      Self::timer_function(Self::clear_timer, state.clone()),
    ).unwrap();

    context.register_global_builtin_callable(
      js_string!(Self::SET_INTERVAL_NAME),
      0,
      Self::timer_function(Self::set_interval, state.clone()),
    ).unwrap();

  }

  fn timer_function(
    f: fn(&JsValue, &[JsValue], &mut Self, &mut boa_engine::Context) -> JsResult<JsValue>,
    state: Rc<RefCell<Self>>,
  ) -> NativeFunction {
    // SAFETY: `Console` doesn't contain types that need tracing.
    unsafe {
      NativeFunction::from_closure(move |this, args, context| {
        f(this, args, &mut state.borrow_mut(), context)
      })
    }
  }

  fn set_timeout(
    _: &JsValue,
    args: &[JsValue],
    timer: &mut Self,
    _context: &mut boa_engine::Context,
  ) -> JsResult<JsValue> {
    let callback = args.get(0).cloned();
    let binding = callback.unwrap();
    let timeout = match args.get(1).cloned().unwrap_or_default().as_number() {
      Some(timeout) => timeout as u64,
      None => 0,
    };

    let arguments = args[2..].to_vec();
    let id = timer.timers.borrow().add_timer(
      Duration::from_millis(timeout),
      TimerType::Timeout,
      TimerJsCallable {
        callable: binding,
        args: arguments,
      },
    );

    Ok(JsValue::new(id))
  }

  fn clear_timer(
    _: &JsValue,
    args: &[JsValue],
    timer: &mut Self,
    _context: &mut boa_engine::Context,
  ) -> JsResult<JsValue> {
    let id = args
      .get(0)
      .cloned()
      .unwrap_or_default()
      .as_number()
      .unwrap_or_default() as u64;

    timer.timers.borrow().clear_timer(&id);
    Ok(JsValue::undefined())
  }

  fn set_interval(
    _: &JsValue,
    args: &[JsValue],
    timer: &mut Self,
    _context: &mut boa_engine::Context,
  ) -> JsResult<JsValue> {
    let callback = args.get(0).cloned();
    let binding = callback.unwrap();
    let timeout = match args.get(1).cloned().unwrap_or_default().as_number() {
      Some(timeout) => timeout as u64,
      None => 0,
    };

    let arguments = args[2..].to_vec();
    let id = timer.timers.borrow().add_timer(
      Duration::from_millis(timeout),
      TimerType::Interval,
      TimerJsCallable {
        callable: binding,
        args: arguments,
      },
    );

    Ok(JsValue::new(id))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use boa_engine::object::FunctionObjectBuilder;
  use boa_engine::{Context, JsResult, NativeFunction};
  use std::future::poll_fn;
  use std::sync::{Arc, Mutex};
  use std::time::Duration;

  fn create_context() -> Context {
    Context::default()
  }

  fn create_callable_js_value(
    context: &mut Context,
    length: usize,
    closure: impl Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + Copy + 'static,
  ) -> JsValue {
    let function = FunctionObjectBuilder::new(context.realm(), unsafe {
      NativeFunction::from_closure(closure)
    })
    .length(length)
    .build();
    JsValue::new(function)
  }

  #[tokio::test]
  async fn test_timers() {
    let timers = TimerQueue::new();
    let mut context = create_context();

    let callable = create_callable_js_value(&mut context, 2, move |_, _args, _| {
      let firt_arg = _args.get(0).unwrap().clone();
      let second_arg = _args.get(1).unwrap().clone();
      assert_eq!(firt_arg, JsValue::new(1), "First argument is not 1");
      assert_eq!(second_arg, JsValue::new(2), "Second argument is not 2");
      println!("Timeout callback");
      Ok(JsValue::new(()))
    });

    let args = vec![JsValue::new(1), JsValue::new(2)];
    let callback = TimerJsCallable { callable, args };

    let id =
      timers.add_timer(Duration::from_secs(1), TimerType::Timeout, callback.clone());
    assert_eq!(id, 1);

    poll_fn(|cx| timers.poll_timers(cx, &mut context)).await;
    context.run_jobs();
  }

  #[tokio::test]
  async fn test_multiple_timers() {
    let timers = Arc::new(Mutex::new(TimerQueue::new()));
    let mut context = create_context();

    let callable = create_callable_js_value(&mut context, 2, move |_, _args, _| {
      let firt_arg = _args.get(0).unwrap().clone();
      assert_eq!(firt_arg, JsValue::new(1), "First argument is not 1");
      println!("Timeout callback");
      Ok(JsValue::new(()))
    });

    let args = vec![JsValue::new(1), JsValue::new(2)];
    let callback = TimerJsCallable { callable, args };

    let id = timers.lock().unwrap().add_timer(
      Duration::from_secs(3),
      TimerType::Timeout,
      callback.clone(),
    );
    assert_eq!(id, 1);

    let future = async {
      tokio::time::sleep(Duration::from_secs(3)).await;
      timers.lock().unwrap().add_timer(
        Duration::from_secs(1),
        TimerType::Timeout,
        callback.clone(),
      );
      timers.lock().unwrap().add_timer(
        Duration::from_secs(2),
        TimerType::Timeout,
        callback.clone(),
      );
    };

    let future2 = async {
      tokio::time::sleep(Duration::from_secs(2)).await;
      timers.lock().unwrap().add_timer(
        Duration::from_secs(1),
        TimerType::Timeout,
        callback.clone(),
      );
      timers.lock().unwrap().add_timer(
        Duration::from_secs(2),
        TimerType::Timeout,
        callback.clone(),
      );
    };

    let poll_fut = poll_fn(|cx| {
      let result = timers.lock().unwrap().poll_timers(cx, &mut context);
      context.run_jobs();
      result
    });


    tokio::join!(future, future2, poll_fut);
    assert!(timers.lock().unwrap().is_empty());
  }

  #[tokio::test]
  async fn test_clear_timers() {
    let timers = Arc::new(Mutex::new(TimerQueue::new()));
    let mut context = create_context();

    let callable = create_callable_js_value(&mut context, 2, move |_, _args, _| {
      let firt_arg = _args.get(0).unwrap().clone();
      assert_eq!(firt_arg, JsValue::new(1), "First argument is not 1");
      println!("Timeout callback");
      Ok(JsValue::new(()))
    });

    let args = vec![JsValue::new(1), JsValue::new(2)];
    let callback = TimerJsCallable { callable, args };

    let id = timers.lock().unwrap().add_timer(
      Duration::from_secs(2),
      TimerType::Timeout,
      callback.clone(),
    );
    assert_eq!(id, 1);

    let future = async {
      tokio::time::sleep(Duration::from_secs(1)).await;
      timers.lock().unwrap().clear_timer(&id);
    };

    let poll_fut = poll_fn(|cx| {
      let result = timers.lock().unwrap().poll_timers(cx, &mut context);
      context.run_jobs();
      result
    });

    tokio::join!(future, poll_fut);
    assert!(timers.lock().unwrap().is_empty());
  }

  #[tokio::test]
  async fn test_10_000_timers() {
    let timers = Arc::new(Mutex::new(TimerQueue::new()));
    let mut context = create_context();

    let callable = create_callable_js_value(&mut context, 2, move |_, _args, _| {
      let firt_arg = _args.get(0).unwrap().clone();
      let second_arg = _args.get(1).unwrap().clone();
      assert_eq!(firt_arg.clone(), JsValue::new(second_arg.as_number().unwrap()/2.0), "Second argument is not 2 times the first argument");
      println!("Timeout callback {}", firt_arg.as_number().unwrap());
      Ok(JsValue::new(()))
    });

    for i in 0..10_000 {
      let args = vec![JsValue::new(i), JsValue::new(i*2)];
      let callback = TimerJsCallable { callable: callable.clone(), args };
      timers.lock().unwrap().add_timer(
        Duration::from_secs(2),
        TimerType::Timeout,
        callback.clone(),
      );
    }

    let future = async {
      tokio::time::sleep(Duration::from_secs(1)).await;
    };

    let poll_fut = poll_fn(|cx| {
      let result = timers.lock().unwrap().poll_timers(cx, &mut context);
      context.run_jobs();
      result
    });

    for i in 0..10_000 {
      let args = vec![JsValue::new(i), JsValue::new(i*2)];
      let callback = TimerJsCallable { callable: callable.clone(), args };
      timers.lock().unwrap().add_timer(
        Duration::from_secs(4),
        TimerType::Timeout,
        callback.clone(),
      );
    }

    tokio::join!(future, poll_fut);
    assert!(timers.lock().unwrap().is_empty());
  }

  #[tokio::test]
  async fn test_timers_interval() {
    let timers = Arc::new(Mutex::new(TimerQueue::new()));
    let mut context = create_context();

    let callable = create_callable_js_value(&mut context, 2, move |_, _args, _| {
      let firt_arg = _args.get(0).unwrap().clone();
      assert_eq!(firt_arg, JsValue::new(1), "First argument is 6");
      println!("Interval callback");
      Ok(JsValue::new(()))
    });

    let args = vec![JsValue::new(1), JsValue::new(2)];
    let callback = TimerJsCallable { callable, args };

    let id = timers.lock().unwrap().add_timer(
      Duration::from_secs(2),
      TimerType::Interval,
      callback.clone(),
    );
    assert_eq!(id, 1);

    let future = async {
      tokio::time::sleep(Duration::from_secs(4)).await;
      timers.lock().unwrap().clear_timer(&id);
    };

    let poll_fut = poll_fn(|cx| {
      let result = timers.lock().unwrap().poll_timers(cx, &mut context);
      context.run_jobs();
      result
    });

    tokio::join!(future, poll_fut);
  }
}
