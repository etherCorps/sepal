//! V8, through the `v8` crate. Compiled in with the `engine-v8` feature.
//!
//! The same engine a browser has, with its JIT: JavaScript-bound work runs
//! several times faster than in QuickJS, and the binary is much larger.
//!
//! `SVELTE_BLITZ_V8_FLAGS` is passed to V8 as it starts, for example
//! `--jitless` to run without generating machine code.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, Once};
use std::time::{Duration, Instant};

use super::{Batch, EventDetail, HostCall, Mods, RUNAWAY_SCRIPT};

type Measure = Box<dyn Fn(u32, &dyn Fn() -> Option<Batch>) -> Option<[f64; 5]>>;

/// What the functions on `__sb_host` work with. Lives in a slot on the
/// isolate, where a native callback can find it.
struct Host {
    start: Instant,
    calls: RefCell<Vec<HostCall>>,
    measure: RefCell<Option<Measure>>,
}

pub struct JsEngine {
    // Before `isolate`: handles have to go before what they point into.
    context: v8::Global<v8::Context>,
    isolate: RefCell<v8::OwnedIsolate>,
    host: Rc<Host>,
    watchdog: Arc<Watchdog>,
}

/// Stops a script that does not return. V8 can only be interrupted from
/// another thread, so there is one: it sleeps until a script starts, and
/// then until that script returns or its time is up.
struct Watchdog {
    limit: Duration,
    /// When the script that is running now started, and a count that tells
    /// one script from the next.
    running: Mutex<(Option<Instant>, u64, bool)>,
    changed: Condvar,
}

impl Watchdog {
    fn watch(self: Arc<Self>, isolate: v8::IsolateHandle) {
        let mut state = self.running.lock().unwrap();
        loop {
            // Nothing is running: wait, for as long as that takes.
            state = self
                .changed
                .wait_while(state, |(since, _, shutdown)| since.is_none() && !*shutdown)
                .unwrap();
            if state.2 {
                return;
            }
            let (Some(since), generation, _) = *state else {
                continue;
            };
            let left = self.limit.saturating_sub(since.elapsed());
            let (next, timeout) = self
                .changed
                .wait_timeout_while(state, left, |(_, current, shutdown)| {
                    *current == generation && !*shutdown
                })
                .unwrap();
            state = next;
            if timeout.timed_out() && state.1 == generation && !state.2 {
                isolate.terminate_execution();
                // Not again for this script; the next one gets its own time.
                state.0 = None;
            }
        }
    }

    fn started(&self) {
        let mut state = self.running.lock().unwrap();
        *state = (Some(Instant::now()), state.1 + 1, false);
        self.changed.notify_one();
    }

    fn returned(&self) {
        let mut state = self.running.lock().unwrap();
        state.0 = None;
        state.1 += 1;
        self.changed.notify_one();
    }
}

fn start_v8() {
    static START: Once = Once::new();
    START.call_once(|| {
        if let Ok(flags) = std::env::var("SVELTE_BLITZ_V8_FLAGS") {
            v8::V8::set_flags_from_string(&flags);
        }
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

impl JsEngine {
    pub fn new() -> Result<Self, String> {
        Self::with_limit(RUNAWAY_SCRIPT)
    }

    pub(super) fn with_limit(limit: Duration) -> Result<Self, String> {
        start_v8();
        let mut isolate = v8::Isolate::new(v8::CreateParams::default());
        // Microtasks run when the host says so (`run_jobs`), as with QuickJS:
        // not at the end of every call, which would be in the middle of
        // Blitz dispatching an event.
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);

        let host = Rc::new(Host {
            start: Instant::now(),
            calls: RefCell::new(Vec::new()),
            measure: RefCell::new(None),
        });
        isolate.set_slot(host.clone());

        let watchdog = Arc::new(Watchdog {
            limit,
            running: Mutex::new((None, 0, false)),
            changed: Condvar::new(),
        });
        let (watcher, handle) = (watchdog.clone(), isolate.thread_safe_handle());
        std::thread::Builder::new()
            .name("svelte-blitz-watchdog".into())
            .spawn(move || watcher.watch(handle))
            .map_err(|e| e.to_string())?;

        let context = {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);

            let object = v8::Object::new(scope);
            let functions = [
                ("log", v8::Function::new(scope, log)),
                ("now", v8::Function::new(scope, now)),
                ("call", v8::Function::new(scope, call)),
                ("measure", v8::Function::new(scope, measure)),
            ];
            for (name, function) in functions {
                let function = function.ok_or("cannot create a host function")?;
                let key = string(scope, name);
                object.set(scope, key.into(), function.into());
            }
            let key = string(scope, "__sb_host");
            context.global(scope).set(scope, key.into(), object.into());
            v8::Global::new(scope, context)
        };

        // `Isolate::new` leaves the isolate entered, and V8 wants isolates
        // left in the reverse order they were entered in. Two documents do
        // not live that way (a dev reload makes the new one before it lets go
        // of the old), so this one is only entered while it is in use.
        unsafe { isolate.exit() };

        Ok(Self {
            context,
            isolate: RefCell::new(isolate),
            host,
            watchdog,
        })
    }

    /// Run `work` inside the engine. Every call into JavaScript goes through
    /// here, so the watchdog times one stretch of script.
    fn enter<R>(&self, work: impl FnOnce(&mut v8::PinScope) -> R) -> R {
        self.watchdog.started();
        let result = {
            let isolate = &mut *self.isolate.borrow_mut();
            // SAFETY: left again below, by the guard; nothing else enters an
            // isolate on this thread in between.
            unsafe { isolate.enter() };
            let isolate = &mut *Entered(isolate);
            v8::scope!(let scope, isolate);
            let context = v8::Local::new(scope, &self.context);
            let scope = &mut v8::ContextScope::new(scope, context);
            let result = work(scope);
            if scope.is_execution_terminating() {
                scope.cancel_terminate_execution();
                eprintln!(
                    "[js error] a script ran for {:?} without returning and was stopped; the app may be in a broken state",
                    self.watchdog.limit
                );
            }
            result
        };
        self.watchdog.returned();
        result
    }

    /// Evaluate a script (the app bundle) and run the microtasks it queued.
    pub fn eval(&self, code: &str) -> Result<(), String> {
        self.enter(|scope| run(scope, code).map(|_| ()))?;
        self.run_jobs();
        Ok(())
    }

    /// Evaluate an expression and return its string value. Handy in tests.
    pub fn eval_string(&self, code: &str) -> Result<String, String> {
        let out = self.enter(|scope| {
            v8::scope!(let scope, scope);
            let value = run(scope, code)?;
            Ok::<_, String>(value.to_rust_string_lossy(scope))
        })?;
        self.run_jobs();
        Ok(out)
    }

    /// Run promise reactions and other queued jobs until the queue is empty.
    pub fn run_jobs(&self) {
        self.enter(|scope| {
            v8::tc_scope!(let scope, scope);
            scope.perform_microtask_checkpoint();
            // Finalizers (`FinalizationRegistry`) are tasks for the platform.
            let platform = v8::V8::get_current_platform();
            while v8::Platform::pump_message_loop(&platform, scope, false) {
                scope.perform_microtask_checkpoint();
            }
            if scope.has_caught() && !scope.has_terminated() {
                eprintln!(
                    "[js error] uncaught in a promise callback: {}",
                    describe(scope)
                );
            }
        });
    }

    /// Deliver a DOM event to the renderer's dispatcher. Returns `DISPATCH_*` flags.
    pub fn dispatch(&self, target: u32, name: &str, bubbles: bool, detail: &EventDetail) -> u32 {
        self.enter(|scope| {
            v8::scope!(let scope, scope);
            let detail = detail_value(scope, detail);
            let args = [
                v8::Integer::new_from_unsigned(scope, target).into(),
                string(scope, name).into(),
                v8::Boolean::new(scope, bubbles).into(),
                detail,
            ];
            entry(scope, &format!("dispatching {name}"), "dispatch", &args)
                .and_then(|flags| flags.uint32_value(scope))
                .unwrap_or(0)
        })
    }

    /// Pull the pending op batch, if anything changed since the last call.
    pub fn take_batch(&self) -> Option<Batch> {
        self.enter(|scope| take_batch_in(scope))
    }

    /// Install `__sb_host.measure(id)`: a forced layout. JS calls it when it
    /// needs to know about a node right now; it returns `[x, y, width,
    /// height, animating]`, or nothing when no layout can be done now.
    pub fn on_measure(
        &self,
        measure: impl Fn(u32, &dyn Fn() -> Option<Batch>) -> Option<[f64; 5]> + 'static,
    ) -> Result<(), String> {
        *self.host.measure.borrow_mut() = Some(Box::new(measure));
        Ok(())
    }

    /// Tell the app the size of the viewport and where the nodes it measures
    /// are now. `boxes` holds the ones that changed: `[id, x, y, width, height, …]`.
    pub fn layout(&self, width: f64, height: f64, boxes: Vec<f64>) {
        self.enter(|scope| {
            v8::scope!(let scope, scope);
            let numbers: Vec<v8::Local<v8::Value>> = boxes
                .iter()
                .map(|number| v8::Number::new(scope, *number).into())
                .collect();
            let args = [
                v8::Number::new(scope, width).into(),
                v8::Number::new(scope, height).into(),
                v8::Array::new_with_elements(scope, &numbers).into(),
            ];
            entry(scope, "reporting the layout", "layout", &args);
        });
    }

    /// The host calls JS has made since the last time this was asked.
    pub fn take_calls(&self) -> Vec<HostCall> {
        std::mem::take(&mut *self.host.calls.borrow_mut())
    }

    /// Settle the promise behind a host call: `Ok` carries the result as JSON,
    /// `Err` the message of the error to reject with.
    pub fn resolve_call(&self, id: u32, result: Result<String, String>) {
        self.enter(|scope| {
            v8::scope!(let scope, scope);
            let (ok, payload) = match &result {
                Ok(json) => (true, json),
                Err(message) => (false, message),
            };
            let args = [
                v8::Integer::new_from_unsigned(scope, id).into(),
                v8::Boolean::new(scope, ok).into(),
                string(scope, payload).into(),
            ];
            entry(scope, "answering a host call", "resolve", &args);
        });
        self.run_jobs();
    }

    /// Run the timers that are due.
    pub fn run_timers(&self) {
        // One timer at a time, with the microtasks it queued run before the
        // next: each is a task of its own, as in a browser.
        loop {
            let ran = self.enter(|scope| {
                v8::scope!(let scope, scope);
                entry(scope, "running timers", "timers", &[])
                    .is_some_and(|ran| ran.boolean_value(scope))
            });
            if !ran {
                break;
            }
            self.run_jobs();
        }
    }

    /// The next timer deadline in host-clock milliseconds, if any timer is pending.
    pub fn next_timer(&self) -> Option<f64> {
        self.enter(|scope| {
            v8::scope!(let scope, scope);
            entry(scope, "reading the next timer", "next_timer", &[])
                .and_then(|next| next.number_value(scope))
                .filter(|next| *next >= 0.0)
        })
    }

    /// The current value of the clock JS sees through `performance.now()`.
    pub fn now(&self) -> f64 {
        self.host.start.elapsed().as_secs_f64() * 1000.0
    }

    /// Force a garbage collection. Finalizers queue `DROP` ops for the next batch.
    pub fn collect_garbage(&self) {
        {
            let isolate = &mut *self.isolate.borrow_mut();
            // SAFETY: as in `enter`.
            unsafe { isolate.enter() };
            Entered(isolate).low_memory_notification();
        }
        self.run_jobs();
    }
}

/// Leaves the isolate when dropped, also when the work in between panics.
struct Entered<'a>(&'a mut v8::OwnedIsolate);

impl std::ops::Deref for Entered<'_> {
    type Target = v8::OwnedIsolate;
    fn deref(&self) -> &v8::OwnedIsolate {
        self.0
    }
}

impl std::ops::DerefMut for Entered<'_> {
    fn deref_mut(&mut self) -> &mut v8::OwnedIsolate {
        self.0
    }
}

impl Drop for Entered<'_> {
    fn drop(&mut self) {
        // SAFETY: entered by whoever made this guard.
        unsafe { self.0.exit() };
    }
}

impl Drop for JsEngine {
    fn drop(&mut self) {
        {
            let mut state = self.watchdog.running.lock().unwrap();
            state.2 = true;
            self.watchdog.changed.notify_one();
        }
        // V8 disposes an isolate from inside it.
        // SAFETY: the isolate is dropped right after this, which leaves it.
        unsafe { self.isolate.get_mut().enter() };
    }
}

fn string<'s>(scope: &v8::PinScope<'s, '_>, text: &str) -> v8::Local<'s, v8::String> {
    v8::String::new(scope, text).unwrap_or_else(|| v8::String::empty(scope))
}

/// What went wrong, from an exception that was caught: its message and stack.
fn describe(scope: &mut v8::PinnedRef<v8::TryCatch<v8::HandleScope>>) -> String {
    let stack = scope
        .stack_trace()
        .or_else(|| scope.exception())
        .map(|value| value.to_rust_string_lossy(scope));
    stack.unwrap_or_else(|| "an exception without a description".to_owned())
}

/// Compile and run a script.
fn run<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    code: &str,
) -> Result<v8::Local<'s, v8::Value>, String> {
    v8::tc_scope!(let tc, scope);
    let source = string(tc, code);
    let result = v8::Script::compile(tc, source, None).and_then(|script| script.run(tc));
    match result {
        Some(value) => Ok(value),
        None if tc.has_terminated() => Err("the script did not return and was stopped".into()),
        None => Err(describe(tc)),
    }
}

/// Call `__sb.<name>(…args)`. Errors are reported and come back as `None`.
fn entry<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    what: &str,
    name: &str,
    args: &[v8::Local<v8::Value>],
) -> Option<v8::Local<'s, v8::Value>> {
    v8::tc_scope!(let tc, scope);
    let global = tc.get_current_context().global(tc);
    let key = string(tc, "__sb");
    let result = global
        .get(tc, key.into())
        .and_then(|sb| sb.to_object(tc))
        .and_then(|sb| {
            let key = string(tc, name);
            let function = v8::Local::<v8::Function>::try_from(sb.get(tc, key.into())?).ok()?;
            function.call(tc, sb.into(), args)
        });
    if result.is_none() && !tc.has_terminated() {
        let message = if tc.has_caught() {
            describe(tc)
        } else {
            format!("`__sb.{name}` is not there to call")
        };
        eprintln!("[js error] while {what}: {message}");
    }
    result
}

/// Take the pending op batch from the renderer, if it has written anything.
fn take_batch_in(scope: &mut v8::PinScope) -> Option<Batch> {
    v8::scope!(let scope, scope);
    let batch = entry(scope, "taking the op batch", "take", &[])?;
    if batch.is_null_or_undefined() {
        return None;
    }
    let batch = batch.to_object(scope)?;
    let key = string(scope, "ops");
    let ops = v8::Local::<v8::Int32Array>::try_from(batch.get(scope, key.into())?).ok()?;
    let mut bytes = vec![0u8; ops.byte_length()];
    ops.copy_contents(&mut bytes);
    let ops = bytes
        .chunks_exact(4)
        .map(|c| i32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let key = string(scope, "strings");
    let list = v8::Local::<v8::Array>::try_from(batch.get(scope, key.into())?).ok()?;
    let strings = (0..list.length())
        .map(|index| {
            list.get_index(scope, index)
                .map(|value| value.to_rust_string_lossy(scope))
                .unwrap_or_default()
        })
        .collect();
    Some(Batch { ops, strings })
}

/// The event's payload as the object JS gets: built from JSON, which is less
/// code than setting each property and fast enough for something this small.
fn detail_value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    detail: &EventDetail,
) -> v8::Local<'s, v8::Value> {
    let with_mods = |mut object: serde_json::Value, mods: &Mods| {
        object["altKey"] = mods.alt.into();
        object["ctrlKey"] = mods.ctrl.into();
        object["metaKey"] = mods.meta.into();
        object["shiftKey"] = mods.shift.into();
        object
    };
    let json = match detail {
        EventDetail::None => return v8::null(scope).into(),
        EventDetail::Pointer {
            client_x,
            client_y,
            page_x,
            page_y,
            offset_x,
            offset_y,
            button,
            buttons,
            mods,
        } => with_mods(
            serde_json::json!({
                "clientX": client_x, "clientY": client_y,
                "pageX": page_x, "pageY": page_y,
                "offsetX": offset_x, "offsetY": offset_y,
                "button": button, "buttons": buttons,
            }),
            mods,
        ),
        EventDetail::Key {
            key,
            code,
            repeat,
            mods,
        } => with_mods(
            serde_json::json!({ "key": key, "code": code, "repeat": repeat }),
            mods,
        ),
        EventDetail::Input { value } => serde_json::json!({ "value": value }),
    };
    let text = string(scope, &json.to_string());
    v8::json::parse(scope, text).unwrap_or_else(|| v8::null(scope).into())
}

fn host_of(scope: &v8::PinScope) -> Option<Rc<Host>> {
    scope.get_slot::<Rc<Host>>().cloned()
}

fn text_argument(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    index: i32,
) -> String {
    args.get(index).to_rust_string_lossy(scope)
}

/// `__sb_host.log(level, message)`
fn log(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _: v8::ReturnValue) {
    let level = args.get(0).uint32_value(scope).unwrap_or(0);
    let message = text_argument(scope, &args, 1);
    match level {
        2 => eprintln!("[js error] {message}"),
        1 => eprintln!("[js warn] {message}"),
        _ => println!("[js] {message}"),
    }
}

/// `__sb_host.now()`
fn now(scope: &mut v8::PinScope, _: v8::FunctionCallbackArguments, mut result: v8::ReturnValue) {
    if let Some(host) = host_of(scope) {
        result.set_double(host.start.elapsed().as_secs_f64() * 1000.0);
    }
}

/// `__sb_host.call(id, name, args)`: only queues, as the QuickJS one does.
fn call(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _: v8::ReturnValue) {
    let Some(host) = host_of(scope) else { return };
    let id = args.get(0).uint32_value(scope).unwrap_or(0);
    let name = text_argument(scope, &args, 1);
    let args = text_argument(scope, &args, 2);
    host.calls.borrow_mut().push(HostCall { id, name, args });
}

/// `__sb_host.measure(id)`
fn measure(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    let Some(host) = host_of(scope) else { return };
    let id = args.get(0).uint32_value(scope).unwrap_or(0);
    let answer = {
        let measure = host.measure.borrow();
        let Some(measure) = measure.as_ref() else {
            return;
        };
        // The callback may ask for the pending batch, which is a call back
        // into JavaScript with this same scope.
        let scope = RefCell::new(&mut *scope);
        measure(id, &|| take_batch_in(&mut scope.borrow_mut()))
    };
    if let Some(numbers) = answer {
        let numbers: Vec<v8::Local<v8::Value>> = numbers
            .iter()
            .map(|number| v8::Number::new(scope, *number).into())
            .collect();
        result.set(v8::Array::new_with_elements(scope, &numbers).into());
    }
}
