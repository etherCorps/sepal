//! Wakes the event loop when the next JS timer is due.
//!
//! There is no frame loop. The UI thread sleeps in the OS until there is input
//! or until this thread fires the waker, and this thread itself sleeps on a
//! condition variable until a deadline exists.

use std::sync::{Arc, Condvar, Mutex};
use std::task::Waker;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default)]
struct State {
    deadline: Option<Instant>,
    waker: Option<Waker>,
    shutdown: bool,
}

pub struct TimerWaker {
    shared: Arc<(Mutex<State>, Condvar)>,
    started: bool,
}

impl Default for TimerWaker {
    fn default() -> Self {
        Self {
            shared: Arc::new((Mutex::new(State::default()), Condvar::new())),
            started: false,
        }
    }
}

impl TimerWaker {
    /// Remember how to wake the event loop. Called from `Document::poll`.
    pub fn set_waker(&mut self, waker: &Waker) {
        let (lock, condvar) = &*self.shared;
        let mut state = lock.lock().unwrap();
        if !state.waker.as_ref().is_some_and(|w| w.will_wake(waker)) {
            state.waker = Some(waker.clone());
            condvar.notify_one();
        }
    }

    /// Set (or clear) the time of the next wake-up.
    pub fn set_deadline(&mut self, in_ms: Option<f64>) {
        let deadline =
            in_ms.map(|ms| Instant::now() + Duration::from_secs_f64(ms.max(0.0) / 1000.0));
        {
            let (lock, condvar) = &*self.shared;
            let mut state = lock.lock().unwrap();
            if state.deadline == deadline {
                return;
            }
            state.deadline = deadline;
            condvar.notify_one();
        }
        if deadline.is_some() && !self.started {
            self.started = true;
            let shared = self.shared.clone();
            thread::Builder::new()
                .name("svelte-blitz-timers".into())
                .spawn(move || run(shared))
                .expect("failed to spawn timer thread");
        }
    }
}

impl Drop for TimerWaker {
    fn drop(&mut self) {
        let (lock, condvar) = &*self.shared;
        lock.lock().unwrap().shutdown = true;
        condvar.notify_one();
    }
}

fn run(shared: Arc<(Mutex<State>, Condvar)>) {
    let (lock, condvar) = &*shared;
    let mut state = lock.lock().unwrap();
    loop {
        if state.shutdown {
            return;
        }
        match (state.deadline, state.waker.is_some()) {
            // Nothing to wait for, or nobody to wake yet: sleep until told otherwise.
            (None, _) | (Some(_), false) => state = condvar.wait(state).unwrap(),
            (Some(deadline), true) => {
                let now = Instant::now();
                if deadline <= now {
                    state.deadline = None;
                    let waker = state.waker.clone();
                    drop(state);
                    if let Some(waker) = waker {
                        waker.wake();
                    }
                    state = lock.lock().unwrap();
                } else {
                    state = condvar.wait_timeout(state, deadline - now).unwrap().0;
                }
            }
        }
    }
}
