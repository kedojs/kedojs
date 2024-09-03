use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use tokio::time::{sleep_until, Duration, Instant, Sleep};

#[derive(Clone)]
pub struct TimerJsCallable {
    pub callable: rust_jsc::JSFunction,
    pub args: Vec<rust_jsc::JSValue>,
}

pub type TimerId = usize;

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
    non_blocking: bool,
}

// Create a mutable sleep struct
pub struct SleepWrapper {
    sleep: RefCell<Option<Sleep>>,
}

pub struct TimerQueue {
    timers: RefCell<BTreeMap<Instant, Vec<TimerData>>>,
    callback_map: RefCell<HashMap<TimerId, TimerJsCallable>>,
    next_timer_id: Cell<TimerId>,
    ref_count: Cell<usize>,
    sleep: Box<SleepWrapper>,
    waker: RefCell<Option<Waker>>,
}

// implement Send for TimerQueue
unsafe impl Send for TimerQueue {}

unsafe impl Sync for TimerQueue {}

impl TimerQueue {
    pub fn new() -> Self {
        Self {
            timers: RefCell::new(BTreeMap::new()),
            callback_map: RefCell::new(HashMap::new()),
            next_timer_id: Cell::new(0),
            ref_count: Cell::new(0),
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
        non_blocking: Option<bool>,
    ) -> TimerId {
        let expiration = Instant::now() + duration;
        let id = self.next_timer_id.get() + 1;
        self.next_timer_id.set(id);

        let non_blocking_flag = non_blocking.unwrap_or(false);
        if !non_blocking_flag {
            // ref_count is used to keep the TimerQueue alive
            self.ref_count.set(self.ref_count.get() + 1);
        }

        let mut timers = self.timers.borrow_mut();
        timers
            .entry(expiration)
            .or_insert_with(Vec::new)
            .push(TimerData {
                id,
                timer_type,
                duration,
                non_blocking: non_blocking_flag,
            });
        // not sure why rust extends the lifetime of the timers borrow
        drop(timers);
        self.callback_map.borrow_mut().insert(id, callback);
        self.update_sleep_timer(expiration);
        id
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.borrow().as_ref() {
            waker.wake_by_ref();
        }
    }

    fn update_sleep_timer(&self, new_expiration: Instant) {
        let mut sleep = self.sleep.sleep.borrow_mut();
        let should_update = match &*sleep {
            Some(existing_sleep) => new_expiration < existing_sleep.deadline(),
            None => true,
        };

        if should_update {
            *sleep = Some(sleep_until(new_expiration));
            self.wake();
        } else {
            // wake if sleep deadline is already expired
            if let Some(sleep) = &*sleep {
                if sleep.deadline() <= Instant::now() {
                    self.wake();
                }
            }
        }
    }

    pub fn clear_timer(&self, id: &TimerId) {
        // TODO: Removing the timeout from the callback map will prevent the callback from being called
        // but the timer will still be in the timers map
        // this will cause the event loop to keep polling the timer until it is removed
        match self.callback_map.borrow_mut().remove(id) {
            Some(callback) => {
                callback.callable.unprotect();
                self.ref_count.set(self.ref_count.get() - 1);
            }
            None => {}
        };

        // remove the timer from the timers map
        let mut timers = self.timers.borrow_mut();
        for (_, timer_data) in timers.iter_mut() {
            timer_data.retain(|timer| timer.id != *id);
        }
    }

    pub fn is_empty(&self) -> bool {
        // println!(
        //     "is_empty: timers: {:?}, ref_count: {:?}",
        //     self.timers.borrow().is_empty(),
        //     self.ref_count.get()
        // );
        // check if the timers map is empty or the ref_count is 0
        self.timers.borrow().is_empty() || self.ref_count.get() == 0
    }

    pub fn poll_timers(&self, cx: &mut Context<'_>) -> Poll<Vec<TimerJsCallable>> {
        // early return if there are no timers
        if self.timers.borrow().is_empty() {
            return Poll::Ready(Vec::new());
        }

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
            let mut ref_counter = self.ref_count.get();

            for timer in data {
                // remove the callback from the callback map if the timer type is Timeout
                if timer.timer_type == TimerType::Timeout {
                    if let Some(callback) =
                        self.callback_map.borrow_mut().remove(&timer.id)
                    {
                        callback.callable.unprotect();
                        tasks.push(callback);
                        // if timer is not non-blocking, decrement the ref_counter
                        if !timer.non_blocking {
                            ref_counter = ref_counter - 1;
                        }
                    }
                } else {
                    // if the timer type is Interval, add the timer back to the timers map
                    // with a new expiration time
                    let expiration = Instant::now() + timer.duration;
                    let timer_id = timer.id;
                    if let Some(callback) = self.callback_map.borrow_mut().get(&timer_id)
                    {
                        timers
                            .entry(expiration)
                            .or_insert_with(Vec::new)
                            .push(timer);
                        tasks.push(callback.clone());
                    }
                };
            }
            // update ref_count
            self.ref_count.set(ref_counter);
        }

        if !timers.is_empty() {
            *self.waker.borrow_mut() = Some(cx.waker().clone());

            let next_expiration = timers.keys().next().unwrap();

            // new sleep from the next expiration
            *self.sleep.sleep.borrow_mut() = Some(sleep_until(*next_expiration));

            let pin = unsafe {
                Pin::new_unchecked(&mut *self.sleep.sleep.borrow_mut().as_mut().unwrap())
                    .poll(cx)
                    .is_ready()
            };

            if pin {
                self.waker.borrow().as_ref().unwrap().wake_by_ref();
            }

            if tasks.is_empty() {
                return Poll::Pending;
            }

            return Poll::Ready(tasks);
        }

        Poll::Ready(tasks)
    }
}

impl<'js> Default for TimerQueue {
    fn default() -> Self {
        Self::new()
    }
}
