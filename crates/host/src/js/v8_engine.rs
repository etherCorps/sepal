//! V8, through the `v8` crate. Compiled in with the `engine-v8` feature.
//!
//! The same engine a browser has, with its JIT: JavaScript-bound work runs
//! several times faster than in QuickJS, and the binary is much larger.
//!
//! `SEPAL_V8_FLAGS` is passed to V8 as it starts, for example
//! `--jitless` to run without generating machine code.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, Once};
use std::time::{Duration, Instant};

use super::{
    Batch, EventDetail, HostCall, Mods, ModuleSource, RUNAWAY_SCRIPT, SCROLL_FIELDS, meta_url,
    resolve_module,
};

type Measure = Box<dyn Fn(u32, &dyn Fn() -> Option<Batch>) -> Option<[f64; 9]>>;

/// What the functions on `__sb_host` work with. Lives in a slot on the
/// isolate, where a native callback can find it.
struct Host {
    start: Instant,
    calls: RefCell<Vec<HostCall>>,
    measure: RefCell<Option<Measure>>,
    /// A promise was rejected with no handler since the last report.
    rejected: std::cell::Cell<bool>,
    /// The time JS sees once the host has taken the clock over (`set_time`).
    time: std::cell::Cell<Option<f64>>,
    /// Where modules come from (`set_modules`).
    source: RefCell<Option<ModuleSource>>,
    /// Every module compiled so far, by name (its URL), and each one's name
    /// by its identity hash, which is all V8's callbacks give.
    // ponytail: identity hashes can collide; a Vec per hash if one ever does.
    modules: RefCell<HashMap<String, v8::Global<v8::Module>>>,
    names: RefCell<HashMap<i32, String>>,
    /// `import()`s that wait for `run_jobs`: the promise to settle, and the
    /// URL to load or why there is none. Never loaded inside the call itself,
    /// which runs in the middle of a script.
    imports: RefCell<Vec<(v8::Global<v8::PromiseResolver>, Result<String, String>)>>,
}

impl Host {
    fn now(&self) -> f64 {
        self.time
            .get()
            .unwrap_or_else(|| self.start.elapsed().as_secs_f64() * 1000.0)
    }
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
        if let Ok(flags) = std::env::var("SEPAL_V8_FLAGS") {
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
            rejected: std::cell::Cell::new(false),
            time: std::cell::Cell::new(None),
            source: RefCell::new(None),
            modules: RefCell::default(),
            names: RefCell::default(),
            imports: RefCell::default(),
        });
        isolate.set_slot(host.clone());
        isolate.set_promise_reject_callback(promise_rejected);
        isolate.set_host_import_module_dynamically_callback(dynamic_import);
        isolate.set_host_initialize_import_meta_object_callback(import_meta);

        let watchdog = Arc::new(Watchdog {
            limit,
            running: Mutex::new((None, 0, false)),
            changed: Condvar::new(),
        });
        let (watcher, handle) = (watchdog.clone(), isolate.thread_safe_handle());
        std::thread::Builder::new()
            .name("sepal-watchdog".into())
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
        self.enter(|scope| run(scope, code, None).map(|_| ()))?;
        self.run_jobs();
        Ok(())
    }

    /// Load modules from `source` from now on, by URL: what `import` and
    /// `import()` reach, and [`run_module`](Self::run_module).
    pub fn set_modules(&self, source: ModuleSource) {
        *self.host.source.borrow_mut() = Some(source);
    }

    /// Run a classic script of the page, as a browser does: not in strict
    /// mode unless it asks, and with `url` as the base of its `import()`s.
    /// The microtasks it queues wait for the caller's `run_jobs`.
    pub fn eval_script(&self, code: &str, url: &str) -> Result<(), String> {
        let result = self.enter(|scope| run(scope, code, Some(url)).map(|_| ()));
        result
    }

    /// Run a module script of the page: the module at `url`, or `inline`
    /// code with that name. What it throws while it runs rejects its
    /// promise, and is reported as an unhandled rejection.
    pub fn run_module(&self, url: &str, inline: Option<&str>) -> Result<(), String> {
        let result = self.enter(|scope| {
            v8::scope!(let scope, scope);
            let module = compile(scope, &self.host, url, inline)?;
            evaluate(scope, module).map(|_| ())
        });
        self.run_jobs();
        result
    }

    /// Evaluate an expression and return its string value. Handy in tests.
    pub fn eval_string(&self, code: &str) -> Result<String, String> {
        let out = self.enter(|scope| {
            v8::scope!(let scope, scope);
            let value = run(scope, code, None)?;
            Ok::<_, String>(value.to_rust_string_lossy(scope))
        })?;
        self.run_jobs();
        Ok(out)
    }

    /// Run promise reactions and other queued jobs until the queue is empty,
    /// and the `import()`s they made, which queue more.
    pub fn run_jobs(&self) {
        loop {
            self.run_microtasks();
            let imports = std::mem::take(&mut *self.host.imports.borrow_mut());
            if imports.is_empty() {
                break;
            }
            self.enter(|scope| {
                for (resolver, target) in imports {
                    settle_import(scope, &self.host, resolver, target);
                }
            });
        }
        if self.host.rejected.replace(false) {
            self.enter(|scope| {
                v8::scope!(let scope, scope);
                entry(scope, "reporting unhandled rejections", "rejections", &[]);
            });
        }
    }

    fn run_microtasks(&self) {
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
    /// height, animating, clientWidth, clientHeight, scrollWidth,
    /// scrollHeight]`, or nothing when no layout can be done now.
    pub fn on_measure(
        &self,
        measure: impl Fn(u32, &dyn Fn() -> Option<Batch>) -> Option<[f64; 9]> + 'static,
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

    /// Run the callbacks the app queued with `requestAnimationFrame`, each
    /// with `time`, the frame's timestamp. As in the QuickJS wrapper.
    pub fn run_frame(&self, time: f64) {
        loop {
            let ran = self.enter(|scope| {
                v8::scope!(let scope, scope);
                let time = v8::Number::new(scope, time).into();
                entry(scope, "running animation frame callbacks", "frame", &[time])
                    .is_some_and(|ran| ran.boolean_value(scope))
            });
            if !ran {
                break;
            }
            self.run_jobs();
        }
    }

    /// Whether the app waits for a frame (`requestAnimationFrame`).
    pub fn frame_pending(&self) -> bool {
        self.enter(|scope| {
            v8::scope!(let scope, scope);
            entry(scope, "asking for a frame", "frame_pending", &[])
                .is_some_and(|pending| pending.boolean_value(scope))
        })
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

    /// Take the clock JS sees away from real time and put it at `ms`. As in
    /// the QuickJS wrapper: for a document whose animation clock is moved by hand.
    pub fn set_time(&self, ms: f64) {
        self.host.time.set(Some(ms));
    }

    /// The current value of the clock JS sees through `performance.now()`.
    pub fn now(&self) -> f64 {
        self.host.now()
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
        // Handles into the isolate go before it does.
        self.host.modules.borrow_mut().clear();
        self.host.imports.borrow_mut().clear();
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

/// Compile and run a script, named `name` (its URL, the base of its `import()`s).
fn run<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    code: &str,
    name: Option<&str>,
) -> Result<v8::Local<'s, v8::Value>, String> {
    v8::tc_scope!(let tc, scope);
    let source = string(tc, code);
    let origin = name.map(|name| origin(tc, name, false));
    let result = v8::Script::compile(tc, source, origin.as_ref()).and_then(|script| script.run(tc));
    match result {
        Some(value) => Ok(value),
        None if tc.has_terminated() => Err("the script did not return and was stopped".into()),
        None => Err(describe(tc)),
    }
}

/// Where a script or module came from: its URL, which is what `import()`
/// and `import.meta` are told.
fn origin<'s>(scope: &v8::PinScope<'s, '_>, name: &str, module: bool) -> v8::ScriptOrigin<'s> {
    let name = string(scope, name);
    v8::ScriptOrigin::new(
        scope,
        name.into(),
        0,
        0,
        false,
        -1,
        None,
        false,
        false,
        module,
        None,
    )
}

/// Compile the module named `name` (its URL) and everything it imports,
/// each once. `inline` is the code of a module script in the page.
fn compile<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    host: &Host,
    name: &str,
    inline: Option<&str>,
) -> Result<v8::Local<'s, v8::Module>, String> {
    if let Some(module) = host.modules.borrow().get(name) {
        return Ok(v8::Local::new(scope, module));
    }
    let code = match inline {
        Some(code) => code.to_owned(),
        None => {
            let source = host.source.borrow();
            source.as_ref().ok_or("this page cannot load modules")?(name)?
        }
    };
    let module = {
        v8::tc_scope!(let tc, scope);
        let text = string(tc, &code);
        let origin = origin(tc, name, true);
        let mut source = v8::script_compiler::Source::new(text, Some(&origin));
        match v8::script_compiler::compile_module(tc, &mut source) {
            Some(module) => module,
            None => return Err(describe(tc)),
        }
    };
    // Before what it imports: an import cycle finds it here.
    let global = v8::Global::new(scope, module);
    host.modules.borrow_mut().insert(name.to_owned(), global);
    host.names
        .borrow_mut()
        .insert(module.get_identity_hash().get(), name.to_owned());
    let requests = module.get_module_requests();
    for index in 0..requests.length() {
        let Some(request) = requests
            .get(scope, index)
            .and_then(|request| v8::Local::<v8::ModuleRequest>::try_from(request).ok())
        else {
            continue;
        };
        let specifier = request.get_specifier().to_rust_string_lossy(scope);
        compile(scope, host, &resolve_module(name, &specifier)?, None)?;
    }
    Ok(module)
}

/// Link a compiled module to what it imports and run it. Returns the
/// promise of its evaluation.
fn evaluate<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    module: v8::Local<'s, v8::Module>,
) -> Result<v8::Local<'s, v8::Value>, String> {
    v8::tc_scope!(let tc, scope);
    if module.get_status() == v8::ModuleStatus::Uninstantiated
        && module.instantiate_module(tc, resolve).is_none()
    {
        return Err(describe(tc));
    }
    match module.evaluate(tc) {
        Some(done) => Ok(done),
        None if tc.has_terminated() => Err("the module did not return and was stopped".into()),
        None => Err(describe(tc)),
    }
}

/// V8 asks for a module that `referrer` imports. Everything was compiled
/// ahead (`compile`), so this only looks it up.
fn resolve<'s>(
    context: v8::Local<'s, v8::Context>,
    specifier: v8::Local<'s, v8::String>,
    _attributes: v8::Local<'s, v8::FixedArray>,
    referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {
    v8::callback_scope!(unsafe scope, context);
    let host = host_of(scope)?;
    let specifier = specifier.to_rust_string_lossy(scope);
    let found = host
        .names
        .borrow()
        .get(&referrer.get_identity_hash().get())
        .cloned()
        .ok_or_else(|| "a module the host does not know imports".to_owned())
        .and_then(|base| resolve_module(&base, &specifier))
        .and_then(|url| {
            host.modules
                .borrow()
                .get(&url)
                .cloned()
                .ok_or(format!("{url} was not loaded"))
        });
    match found {
        Ok(module) => Some(v8::Local::new(scope, module)),
        Err(message) => {
            let message = string(scope, &message);
            let error = v8::Exception::error(scope, message);
            scope.throw_exception(error);
            None
        }
    }
}

/// `import()`: a promise now, the module at the next `run_jobs`.
fn dynamic_import<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _options: v8::Local<'s, v8::Data>,
    resource: v8::Local<'s, v8::Value>,
    specifier: v8::Local<'s, v8::String>,
    _attributes: v8::Local<'s, v8::FixedArray>,
) -> Option<v8::Local<'s, v8::Promise>> {
    let resolver = v8::PromiseResolver::new(scope)?;
    let promise = resolver.get_promise(scope);
    let host = host_of(scope)?;
    let base = resource.to_rust_string_lossy(scope);
    let target = resolve_module(&base, &specifier.to_rust_string_lossy(scope));
    let resolver = v8::Global::new(scope, resolver);
    host.imports.borrow_mut().push((resolver, target));
    Some(promise)
}

/// Load what an `import()` asked for and settle its promise: with the
/// module's namespace once it has run, or with why it could not be had.
fn settle_import(
    scope: &mut v8::PinScope,
    host: &Host,
    resolver: v8::Global<v8::PromiseResolver>,
    target: Result<String, String>,
) {
    v8::scope!(let scope, scope);
    let resolver = v8::Local::new(scope, resolver);
    let loaded = target.and_then(|url| {
        let module = compile(scope, host, &url, None)?;
        let done = evaluate(scope, module)?;
        Ok((module, done))
    });
    let settled = match loaded {
        Ok((module, done)) => {
            // Once the module has run: a module with top-level `await` is
            // not done when `evaluate` returns.
            let namespace = module.get_module_namespace();
            let give = v8::Function::builder(
                |_: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut out: v8::ReturnValue| { out.set(args.data()) },
            )
            .data(namespace)
            .build(scope);
            v8::Local::<v8::Promise>::try_from(done)
                .ok()
                .zip(give)
                .and_then(|(done, give)| done.then(scope, give))
                .and_then(|namespace| resolver.resolve(scope, namespace.into()))
        }
        Err(message) => {
            let message = string(scope, &message);
            let error = v8::Exception::error(scope, message);
            resolver.reject(scope, error)
        }
    };
    if settled.is_none() {
        eprintln!("[js error] an import() could not be settled");
    }
}

/// `import.meta` of a module: its `url`.
extern "C" fn import_meta(
    context: v8::Local<v8::Context>,
    module: v8::Local<v8::Module>,
    meta: v8::Local<v8::Object>,
) {
    v8::callback_scope!(unsafe scope, context);
    v8::scope!(let scope, scope);
    let Some(host) = host_of(scope) else { return };
    let Some(name) = host
        .names
        .borrow()
        .get(&module.get_identity_hash().get())
        .cloned()
    else {
        return;
    };
    let key = string(scope, "url");
    let value = string(scope, meta_url(&name));
    meta.create_data_property(scope, key.into(), value.into());
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
        EventDetail::Scroll(metrics) => SCROLL_FIELDS
            .iter()
            .zip(metrics)
            .map(|(name, value)| (name.to_string(), serde_json::json!(value)))
            .collect::<serde_json::Map<_, _>>()
            .into(),
    };
    let text = string(scope, &json.to_string());
    v8::json::parse(scope, text).unwrap_or_else(|| v8::null(scope).into())
}

/// V8 says a promise was rejected with no handler, or that one has been added
/// since. The bookkeeping is in JS (shims.js); `run_jobs` asks for the report.
extern "C" fn promise_rejected(message: v8::PromiseRejectMessage) {
    let handled = match message.get_event() {
        v8::PromiseRejectEvent::PromiseRejectWithNoHandler => false,
        v8::PromiseRejectEvent::PromiseHandlerAddedAfterReject => true,
        _ => return,
    };
    v8::callback_scope!(unsafe scope, &message);
    v8::scope!(let scope, scope);
    let Some(host) = host_of(scope) else { return };
    let global = scope.get_current_context().global(scope);
    let key = string(scope, "__sb");
    // Before the bundle has set `__sb` up there is nobody to tell.
    if !global
        .get(scope, key.into())
        .is_some_and(|sb| sb.is_object())
    {
        return;
    }
    let reason = message
        .get_value()
        .unwrap_or_else(|| v8::undefined(scope).into());
    let args = [
        message.get_promise().into(),
        reason,
        v8::Boolean::new(scope, handled).into(),
    ];
    if entry(scope, "noting a rejected promise", "rejection", &args).is_some() && !handled {
        host.rejected.set(true);
    }
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
        result.set_double(host.now());
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
