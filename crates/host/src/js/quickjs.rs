//! QuickJS-ng, through `rquickjs`. The default engine: small, and an
//! interpreter.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rquickjs::context::EvalOptions;
use rquickjs::loader::{ImportAttributes, Loader, Resolver};
use rquickjs::module::Declared;
use rquickjs::{
    CatchResultExt, Context, Ctx, Exception, Function, Module, Object, Persistent, Promise,
    Runtime, TypedArray, Value, WriteOptions,
};

use super::{
    Batch, Compiled, EventDetail, HostCall, Mods, ModuleBytecode, ModuleSource, RUNAWAY_SCRIPT,
    SCROLL_FIELDS, meta_url, resolve_module,
};
use crate::bytecode::{self, Header};
use crate::net::AppFolder;

/// How much stack a script may use before it is stopped with "Maximum call
/// stack size exceeded".
///
/// QuickJS's own limit is 1 MB, which is about a thousand calls deep, and
/// fewer on Windows, where its frames are larger: there, the components of a
/// Bits UI page nest deeper than that. The UI thread has 8 MB (`build.rs`
/// sees to it on Windows), so this leaves as much again for the host.
const SCRIPT_STACK: usize = 4 * 1024 * 1024;

pub struct JsEngine {
    runtime: Runtime,
    context: Context,
    calls: Rc<RefCell<Vec<HostCall>>>,
    /// When the host last handed control to JavaScript. Read by the watchdog.
    entered: Rc<Cell<Instant>>,
    /// A promise was rejected with no handler since the last report.
    rejected: Rc<Cell<bool>>,
    /// The time JS sees, in milliseconds, once the host has taken the clock
    /// over (`set_time`). Until then it is the real time since the start.
    time: Rc<Cell<Option<f64>>>,
}

impl JsEngine {
    pub fn new() -> Result<Self, String> {
        Self::with_limit(RUNAWAY_SCRIPT)
    }

    pub(super) fn with_limit(limit: Duration) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
        runtime.set_max_stack_size(SCRIPT_STACK);
        let entered = Rc::new(Cell::new(Instant::now()));
        let since = entered.clone();
        // The engine calls this every few thousand operations while a script
        // runs. Returning `true` stops the script with an error it cannot catch.
        runtime.set_interrupt_handler(Some(Box::new(move || {
            if since.get().elapsed() < limit {
                return false;
            }
            // Start over, so the report and whatever runs next get their own time.
            since.set(Instant::now());
            eprintln!(
                "[js error] a script ran for {:?} without returning and was stopped; the app may be in a broken state",
                limit
            );
            true
        })));
        // Promises rejected with nobody listening. The bookkeeping is in JS
        // (shims.js); `run_jobs` asks for the report once microtasks are done.
        let rejected = Rc::new(Cell::new(false));
        let noted = rejected.clone();
        runtime.set_host_promise_rejection_tracker(Some(Box::new(
            move |ctx, promise, reason, handled| {
                // Before the bundle has set `__sb` up there is nobody to tell.
                let Ok(track) = entry(&ctx).and_then(|sb| sb.get::<_, Function>("rejection"))
                else {
                    return;
                };
                if track.call::<_, ()>((promise, reason, handled)).is_ok() && !handled {
                    noted.set(true);
                }
            },
        )));
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        let start = Instant::now();
        let time = Rc::new(Cell::new(None));
        let set = time.clone();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let queue = calls.clone();

        context
            .with(|ctx| -> rquickjs::Result<()> {
                let host = Object::new(ctx.clone())?;

                host.set(
                    "log",
                    Function::new(ctx.clone(), |level: i32, message: String| match level {
                        2 => eprintln!("[js error] {message}"),
                        1 => eprintln!("[js warn] {message}"),
                        _ => println!("[js] {message}"),
                    })?,
                )?;

                host.set(
                    "now",
                    Function::new(ctx.clone(), move || -> f64 {
                        set.get()
                            .unwrap_or_else(|| start.elapsed().as_secs_f64() * 1000.0)
                    })?,
                )?;

                host.set(
                    "call",
                    Function::new(ctx.clone(), move |id: u32, name: String, args: String| {
                        queue.borrow_mut().push(HostCall { id, name, args });
                    })?,
                )?;

                ctx.globals().set("__sb_host", host)?;
                Ok(())
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            runtime,
            context,
            calls,
            entered,
            rejected,
            time,
        })
    }

    /// Every call into JavaScript goes through here, so the watchdog times
    /// one stretch of script and not the whole life of the app.
    fn enter(&self) -> &Context {
        self.entered.set(Instant::now());
        &self.context
    }

    /// Evaluate a script (the app bundle) and run the microtasks it queued.
    pub fn eval(&self, code: &str) -> Result<(), String> {
        self.enter().with(|ctx| {
            ctx.eval::<(), _>(code)
                .catch(&ctx)
                .map_err(|e| e.to_string())
        })?;
        self.run_jobs();
        Ok(())
    }

    /// Load modules from `source` from now on, by URL: what `import` and
    /// `import()` reach, and [`run_module`](Self::run_module). A module that
    /// `bytecode` has bytecode for is loaded from that instead.
    pub fn set_modules(&self, source: ModuleSource, bytecode: Option<ModuleBytecode>) {
        self.runtime.set_loader(Urls, Files { source, bytecode });
    }

    /// Run a classic script of the page, as a browser does: not in strict
    /// mode unless it asks, and with `url` as the base of its `import()`s.
    /// The microtasks it queues wait for the caller's `run_jobs`.
    pub fn eval_script(&self, code: &str, url: &str) -> Result<(), String> {
        let mut options = EvalOptions::default();
        options.strict = false;
        options.filename = Some(url.to_owned());
        let result = self.enter().with(|ctx| {
            ctx.eval_with_options::<(), _>(code, options)
                .catch(&ctx)
                .map_err(|e| e.to_string())
        });
        result
    }

    /// Run a module script of the page: the module at `url`, or `inline`
    /// code with that name. What it throws while it runs rejects its
    /// promise, and is reported as an unhandled rejection.
    pub fn run_module(&self, url: &str, inline: Option<&str>) -> Result<(), String> {
        let started = self
            .enter()
            .with(|ctx| -> Result<Persistent<Promise<'static>>, String> {
                let promise = match inline {
                    Some(code) => {
                        let module = Module::declare(ctx.clone(), url, code)
                            .and_then(|module| {
                                module.meta()?.set("url", meta_url(url))?;
                                Ok(module)
                            })
                            .catch(&ctx)
                            .map_err(|e| e.to_string())?;
                        let (_, promise) = module.eval().catch(&ctx).map_err(|e| e.to_string())?;
                        promise
                    }
                    None => Module::import(&ctx, url)
                        .catch(&ctx)
                        .map_err(|e| e.to_string())?,
                };
                Ok(Persistent::save(&ctx, promise))
            })?;
        self.run_jobs();
        // Kept until here so the promise is not collected while it runs.
        drop(started);
        Ok(())
    }

    /// Evaluate an expression and return its string value. Handy in tests.
    pub fn eval_string(&self, code: &str) -> Result<String, String> {
        let out = self.enter().with(|ctx| {
            ctx.eval::<String, _>(code)
                .catch(&ctx)
                .map_err(|e| e.to_string())
        })?;
        self.run_jobs();
        Ok(out)
    }

    /// Run promise reactions and other queued jobs until the queue is empty.
    pub fn run_jobs(&self) {
        self.entered.set(Instant::now());
        loop {
            match self.runtime.execute_pending_job() {
                Ok(true) => continue,
                Ok(false) => break,
                Err(error) => {
                    // The job's context holds the exception that was thrown.
                    let message = error.0.with(|ctx| {
                        let exception = ctx.catch();
                        match exception.as_exception() {
                            Some(e) => format!(
                                "{}\n{}",
                                e.message().unwrap_or_default(),
                                e.stack().unwrap_or_default()
                            ),
                            None => format!("{exception:?}"),
                        }
                    });
                    eprintln!("[js error] uncaught in a promise callback: {message}");
                }
            }
        }
        if self.rejected.replace(false) {
            let reported = self.context.with(|ctx| -> rquickjs::Result<()> {
                let report: Function = entry(&ctx)?.get("rejections")?;
                report.call(())
            });
            if let Err(error) = reported {
                self.report("reporting unhandled rejections", error);
            }
        }
    }

    /// Deliver a DOM event to the renderer's dispatcher. Returns `DISPATCH_*` flags.
    pub fn dispatch(&self, target: u32, name: &str, bubbles: bool, detail: &EventDetail) -> u32 {
        let result = self.enter().with(|ctx| -> rquickjs::Result<u32> {
            let dispatch: Function = entry(&ctx)?.get("dispatch")?;
            let detail = detail_object(&ctx, detail)?;
            dispatch.call((target, name, bubbles, detail))
        });
        match result {
            Ok(flags) => flags,
            Err(error) => {
                self.report(&format!("dispatching {name}"), error);
                0
            }
        }
    }

    /// Pull the pending op batch, if anything changed since the last call.
    pub fn take_batch(&self) -> Option<Batch> {
        match self.enter().with(|ctx| take_batch_in(&ctx)) {
            Ok(batch) => batch,
            Err(error) => {
                self.report("taking the op batch", error);
                None
            }
        }
    }

    /// Install `__sb_host.measure(id)`: a forced layout. JS calls it when it
    /// needs to know about a node right now; it returns `[x, y, width,
    /// height, animating, clientWidth, clientHeight, scrollWidth,
    /// scrollHeight]`: the node's box, for how many more seconds an animation
    /// runs on it, and what it scrolls over; or nothing when no layout can be
    /// done at this moment.
    ///
    /// `measure` gets the node's id and a way to take what the app has
    /// written so far, which has to be applied before laying out.
    pub fn on_measure(
        &self,
        measure: impl Fn(u32, &dyn Fn() -> Option<Batch>) -> Option<[f64; 9]> + 'static,
    ) -> Result<(), String> {
        self.context
            .with(|ctx| -> rquickjs::Result<()> {
                let host: Object = ctx.globals().get("__sb_host")?;
                host.set(
                    "measure",
                    Function::new(ctx.clone(), move |ctx: Ctx<'_>, id: u32| {
                        let take = || match take_batch_in(&ctx) {
                            Ok(batch) => batch,
                            Err(error) => {
                                eprintln!("[js error] taking the op batch: {error}");
                                None
                            }
                        };
                        measure(id, &take).map(|rect| rect.to_vec())
                    })?,
                )
            })
            .map_err(|e| e.to_string())
    }

    /// Tell the app the size of the viewport and where the nodes it measures
    /// are now. `boxes` holds the ones that changed: `[id, x, y, width, height, …]`.
    pub fn layout(&self, width: f64, height: f64, boxes: Vec<f64>) {
        let result = self.enter().with(|ctx| -> rquickjs::Result<()> {
            let layout: Function = entry(&ctx)?.get("layout")?;
            layout.call::<_, ()>((width, height, boxes))
        });
        if let Err(error) = result {
            self.report("reporting the layout", error);
        }
    }

    /// The host calls JS has made since the last time this was asked.
    pub fn take_calls(&self) -> Vec<HostCall> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }

    /// Settle the promise behind a host call: `Ok` carries the result as JSON,
    /// `Err` the message of the error to reject with.
    pub fn resolve_call(&self, id: u32, result: Result<String, String>) {
        let outcome = self.enter().with(|ctx| -> rquickjs::Result<()> {
            let resolve: Function = entry(&ctx)?.get("resolve")?;
            let (ok, payload) = match result {
                Ok(json) => (true, json),
                Err(message) => (false, message),
            };
            resolve.call((id, ok, payload))
        });
        if let Err(error) = outcome {
            self.report("answering a host call", error);
        }
        self.run_jobs();
    }

    /// Run the timers that are due.
    pub fn run_timers(&self) {
        // One timer at a time, with the microtasks it queued run before the
        // next: each is a task of its own, as in a browser.
        loop {
            let result = self.enter().with(|ctx| -> rquickjs::Result<bool> {
                let timers: Function = entry(&ctx)?.get("timers")?;
                timers.call(())
            });
            match result {
                Ok(true) => self.run_jobs(),
                Ok(false) => break,
                Err(error) => {
                    self.report("running timers", error);
                    break;
                }
            }
        }
    }

    /// Run the callbacks the app queued with `requestAnimationFrame`, each
    /// with `time`, the frame's timestamp. One at a time, with the microtasks
    /// each queued run before the next, as with timers.
    pub fn run_frame(&self, time: f64) {
        loop {
            let result = self.enter().with(|ctx| -> rquickjs::Result<bool> {
                let frame: Function = entry(&ctx)?.get("frame")?;
                frame.call((time,))
            });
            match result {
                Ok(true) => self.run_jobs(),
                Ok(false) => break,
                Err(error) => {
                    self.report("running animation frame callbacks", error);
                    break;
                }
            }
        }
    }

    /// Whether the app waits for a frame (`requestAnimationFrame`).
    pub fn frame_pending(&self) -> bool {
        let result = self.enter().with(|ctx| -> rquickjs::Result<bool> {
            let pending: Function = entry(&ctx)?.get("frame_pending")?;
            pending.call(())
        });
        result.unwrap_or_else(|error| {
            self.report("asking for a frame", error);
            false
        })
    }

    /// The next timer deadline in host-clock milliseconds, if any timer is pending.
    pub fn next_timer(&self) -> Option<f64> {
        let result = self.enter().with(|ctx| -> rquickjs::Result<f64> {
            let next_timer: Function = entry(&ctx)?.get("next_timer")?;
            next_timer.call(())
        });
        match result {
            Ok(next) if next >= 0.0 => Some(next),
            Ok(_) => None,
            Err(error) => {
                self.report("reading the next timer", error);
                None
            }
        }
    }

    /// Take the clock JS sees (`performance.now()`, timers, frames) away from real
    /// time and put it at `ms`. From then on it only moves when this is called.
    ///
    /// For a document that is not in a window: its animations run on a clock
    /// the caller moves, and a script that waits for one with a timer has to
    /// be on the same clock, or a slow machine lets the timer win.
    pub fn set_time(&self, ms: f64) {
        self.time.set(Some(ms));
    }

    /// The current value of the clock JS sees through `performance.now()`.
    pub fn now(&self) -> f64 {
        self.context
            .with(|ctx| -> rquickjs::Result<f64> {
                let host: Object = ctx.globals().get("__sb_host")?;
                let now: Function = host.get("now")?;
                now.call(())
            })
            .unwrap_or(0.0)
    }

    /// Force a garbage collection. Finalizers queue `DROP` ops for the next batch.
    pub fn collect_garbage(&self) {
        self.runtime.run_gc();
        self.run_jobs();
    }

    fn report(&self, what: &str, error: rquickjs::Error) {
        let message = self.enter().with(|ctx| {
            if error.is_exception() {
                let exception = ctx.catch();
                match exception.as_exception() {
                    Some(e) => format!(
                        "{}\n{}",
                        e.message().unwrap_or_default(),
                        e.stack().unwrap_or_default()
                    ),
                    None => format!("{exception:?}"),
                }
            } else {
                error.to_string()
            }
        });
        eprintln!("[js error] while {what}: {message}");
    }
}

/// Resolves `import` specifiers to URLs in the app folder.
struct Urls;

impl Resolver for Urls {
    fn resolve<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        resolve_module(base, name).map_err(|message| Exception::throw_message(ctx, &message))
    }
}

/// Reads modules, by URL, from their bytecode when the app folder has some
/// that fits and from their source otherwise, and gives each its
/// `import.meta.url`.
struct Files {
    source: ModuleSource,
    bytecode: Option<ModuleBytecode>,
}

impl Loader for Files {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        if let Some(compiled) = self.bytecode.as_ref().and_then(|find| find(name)) {
            // SAFETY: QuickJS runs bytecode unchecked. This came from the app
            // folder, as the module's source would, and its header says this
            // host wrote it, for this module (`bytecode::lookup`). What
            // QuickJS does check (its format version, a checksum) turns a
            // damaged file into an error, which falls back to the source.
            match unsafe { Module::load(ctx.clone(), &compiled) } {
                Ok(module) => {
                    module.meta()?.set("url", meta_url(name))?;
                    return Ok(module);
                }
                Err(error) => {
                    let reason = caught(ctx, error);
                    let path = name.trim_start_matches(crate::net::ORIGIN.trim_end_matches('/'));
                    eprintln!(
                        "[host] {path}{} did not load ({reason}): loading the JavaScript instead",
                        bytecode::SUFFIX
                    );
                }
            }
        }
        let source =
            (self.source)(name).map_err(|message| Exception::throw_message(ctx, &message))?;
        let module = Module::declare(ctx.clone(), name, source)?;
        module.meta()?.set("url", meta_url(name))?;
        Ok(module)
    }
}

/// What a failed call threw, as text.
fn caught(ctx: &Ctx<'_>, error: rquickjs::Error) -> String {
    if !error.is_exception() {
        return error.to_string();
    }
    let exception = ctx.catch();
    match exception.as_exception() {
        Some(e) => e.message().unwrap_or_default(),
        None => format!("{exception:?}"),
    }
}

/// The engine compiled modules record: "QuickJS-ng 0.16.2".
pub fn engine() -> String {
    // SAFETY: a static, NUL-terminated string.
    let version = unsafe { std::ffi::CStr::from_ptr(rquickjs::qjs::JS_GetVersion()) };
    format!("QuickJS-ng {}", version.to_string_lossy())
}

/// Compile every module of the app folder `dir` to bytecode, next to it
/// (see `bytecode.rs`), with this host's own QuickJS. A module whose
/// bytecode is already what this would write is left alone, and the
/// compiled files of modules that are gone are removed: run after every
/// build, it compiles what changed.
///
/// Nothing runs: modules are compiled, not evaluated. A module that does
/// not compile (a syntax error, a missing import, a script that is not a
/// module) gets no bytecode and loads as JavaScript, as it would have.
pub fn compile_folder(dir: &Path) -> Result<Compiled, String> {
    let root = dir
        .canonicalize()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    let folder = Arc::new(AppFolder::new(Some(&root)));
    let (modules, compiled_files) = bytecode::scan(&root)?;
    let mut done = Compiled::default();

    for file in &compiled_files {
        let module = file.to_string_lossy();
        let module = module.strip_suffix(bytecode::SUFFIX).unwrap_or(&module);
        if !root.join(module).is_file() {
            std::fs::remove_file(root.join(file))
                .map_err(|e| format!("{}: {e}", file.display()))?;
            done.removed += 1;
            // A build that kept only bytecode in a directory it no longer
            // writes (`clear_but_bytecode`) leaves it empty now.
            let mut dir = root.join(file);
            while dir.pop() && dir != root && std::fs::remove_dir(&dir).is_ok() {}
        }
    }

    let mut needed = BTreeSet::new();
    for file in &modules {
        let url = bytecode::url_of(file)?;
        let source = match folder.module(&url) {
            Ok(source) => source,
            Err(reason) => {
                done.left.push((file.to_string_lossy().into_owned(), reason));
                continue;
            }
        };
        if bytecode::current(&folder, &url, source.as_bytes()) {
            done.current += 1;
        } else {
            needed.insert(url);
        }
    }
    if needed.is_empty() {
        return Ok(done);
    }

    let written = Rc::new(RefCell::new(BTreeSet::new()));
    let failures = Rc::new(RefCell::new(Vec::new()));
    let runtime = Runtime::new().map_err(|e| e.to_string())?;
    runtime.set_max_stack_size(SCRIPT_STACK);
    // Compiling a module compiles what it imports, through the loader:
    // the needed ones are written as they go by.
    runtime.set_loader(
        Urls,
        Compiler {
            folder: folder.clone(),
            needed: needed.clone(),
            written: written.clone(),
            failures: failures.clone(),
        },
    );
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        for url in &needed {
            if written.borrow().contains(url) {
                continue;
            }
            let outcome = folder.module(url).and_then(|source| {
                let module = Module::declare(ctx.clone(), url.as_str(), source.clone())
                    .map_err(|error| caught(&ctx, error))?;
                write_compiled(&folder, url, &source, &module)
            });
            match outcome {
                Ok(()) => {
                    written.borrow_mut().insert(url.clone());
                }
                Err(reason) => failures.borrow_mut().push((url.clone(), reason)),
            }
        }
    });

    for (url, reason) in failures.take() {
        if written.borrow().contains(&url) {
            continue;
        }
        // Its old bytecode, if any, would be older than the module now.
        if let Ok(file) = compiled_path(&folder, &url) {
            let _ = std::fs::remove_file(file);
        }
        let path = url
            .strip_prefix(crate::net::ORIGIN)
            .unwrap_or(&url)
            .to_owned();
        if !done.left.iter().any(|(left, _)| *left == path) {
            done.left.push((path, reason));
        }
    }
    done.compiled = written.borrow().len();
    Ok(done)
}

/// Where the bytecode of the module at `url` goes: next to its file.
fn compiled_path(folder: &AppFolder, url: &str) -> Result<std::path::PathBuf, String> {
    let url = blitz_traits::net::Url::parse(url).map_err(|e| format!("{url}: {e}"))?;
    let mut path = folder
        .file(&url)
        .map_err(|e| e.to_string())?
        .into_os_string();
    path.push(bytecode::SUFFIX);
    Ok(path.into())
}

/// Write the bytecode of a module just compiled, without its source text:
/// a third of the size, and error stacks keep their file, line and column.
/// `Function.prototype.toString` gives the `[native code]` form instead.
fn write_compiled(
    folder: &AppFolder,
    url: &str,
    source: &str,
    module: &Module<'_, Declared>,
) -> Result<(), String> {
    let payload = module
        .write(WriteOptions {
            strip_source: true,
            ..Default::default()
        })
        .map_err(|e| format!("cannot write its bytecode: {e}"))?;
    let file = Header::new(url, source.as_bytes()).encode(&payload);
    bytecode::write(&compiled_path(folder, url)?, &file)
}

/// The loader `compile_folder` compiles with: every import from source,
/// and the bytecode of the needed ones written as they are compiled.
struct Compiler {
    folder: Arc<AppFolder>,
    needed: BTreeSet<String>,
    written: Rc<RefCell<BTreeSet<String>>>,
    failures: Rc<RefCell<Vec<(String, String)>>>,
}

impl Loader for Compiler {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        let source = self
            .folder
            .module(name)
            .map_err(|message| Exception::throw_message(ctx, &message))?;
        let module = Module::declare(ctx.clone(), name, source.clone())?;
        if self.needed.contains(name) && !self.written.borrow().contains(name) {
            match write_compiled(&self.folder, name, &source, &module) {
                Ok(()) => {
                    self.written.borrow_mut().insert(name.to_owned());
                }
                Err(reason) => self.failures.borrow_mut().push((name.to_owned(), reason)),
            }
        }
        Ok(module)
    }
}

/// Take the pending op batch from the renderer, if it has written anything.
fn take_batch_in(ctx: &Ctx<'_>) -> rquickjs::Result<Option<Batch>> {
    let take: Function = entry(ctx)?.get("take")?;
    let value: Value = take.call(())?;
    let Some(batch) = value.as_object() else {
        return Ok(None);
    };
    let ops: TypedArray<i32> = batch.get("ops")?;
    let strings: Vec<String> = batch.get("strings")?;
    // Copy the Int32Array out of the JS heap. Going through bytes keeps this
    // correct whatever the alignment of the backing buffer.
    let ops = match ops.as_raw() {
        // SAFETY: the buffer is alive for this call: `ops` holds a reference
        // to it and no JS runs while we read.
        Some(raw) => unsafe { raw.as_ref() }
            .chunks_exact(4)
            .map(|c| i32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        None => Vec::new(),
    };
    Ok(Some(Batch { ops, strings }))
}

/// `globalThis.__sb`, installed by the `sepal` package.
fn entry<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
    ctx.globals().get("__sb")
}

fn detail_object<'js>(ctx: &Ctx<'js>, detail: &EventDetail) -> rquickjs::Result<Value<'js>> {
    let set_mods = |object: &Object<'js>, mods: &Mods| -> rquickjs::Result<()> {
        object.set("altKey", mods.alt)?;
        object.set("ctrlKey", mods.ctrl)?;
        object.set("metaKey", mods.meta)?;
        object.set("shiftKey", mods.shift)?;
        Ok(())
    };

    match detail {
        EventDetail::None => Ok(Value::new_null(ctx.clone())),
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
        } => {
            let object = Object::new(ctx.clone())?;
            object.set("clientX", *client_x)?;
            object.set("clientY", *client_y)?;
            object.set("pageX", *page_x)?;
            object.set("pageY", *page_y)?;
            object.set("offsetX", *offset_x)?;
            object.set("offsetY", *offset_y)?;
            object.set("button", *button)?;
            object.set("buttons", *buttons)?;
            set_mods(&object, mods)?;
            Ok(object.into_value())
        }
        EventDetail::Key {
            key,
            code,
            repeat,
            mods,
        } => {
            let object = Object::new(ctx.clone())?;
            object.set("key", key.as_str())?;
            object.set("code", code.as_str())?;
            object.set("repeat", *repeat)?;
            set_mods(&object, mods)?;
            Ok(object.into_value())
        }
        EventDetail::Input { value } => {
            let object = Object::new(ctx.clone())?;
            object.set("value", value.as_str())?;
            Ok(object.into_value())
        }
        EventDetail::Scroll(metrics) => {
            let object = Object::new(ctx.clone())?;
            for (name, value) in SCROLL_FIELDS.iter().zip(metrics) {
                object.set(*name, *value)?;
            }
            Ok(object.into_value())
        }
    }
}
