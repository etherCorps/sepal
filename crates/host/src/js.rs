//! The embedded JavaScript engine (QuickJS-ng through `rquickjs`).
//!
//! The surface between Rust and JS is deliberately tiny:
//!
//! * JS calls into Rust through `__sb_host` — logging, a clock, and `call`,
//!   which asks the host for something that takes time (see [`HostCall`]).
//! * Rust calls into JS through `__sb` — `dispatch`, `take`, `timers`,
//!   `next_timer`, and `resolve` to answer a `call`.
//!
//! Tree mutations never cross this boundary one at a time. They are pulled as
//! one [`Batch`] per turn by [`JsEngine::take_batch`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::{CatchResultExt, Context, Ctx, Function, Object, Runtime, TypedArray, Value};

/// One turn's worth of tree mutations. See `packages/core/src/protocol.js`.
pub struct Batch {
    pub ops: Vec<i32>,
    pub strings: Vec<String>,
}

/// The payload of a DOM event, turned into plain properties on the JS event object.
pub enum EventDetail {
    None,
    Pointer {
        client_x: f32,
        client_y: f32,
        page_x: f32,
        page_y: f32,
        offset_x: f32,
        offset_y: f32,
        button: i32,
        buttons: u32,
        mods: Mods,
    },
    Key {
        key: String,
        code: String,
        repeat: bool,
        mods: Mods,
    },
    Input {
        value: String,
    },
}

#[derive(Clone, Copy, Default)]
pub struct Mods {
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

/// A request from JS for work the host does off the UI thread (an update
/// check, a download). JS gets a promise; the host answers later with
/// [`JsEngine::resolve_call`].
///
/// Making the request only queues it. Nothing happens, and nothing in the
/// document is touched, until the host picks the queue up at the end of the turn.
pub struct HostCall {
    pub id: u32,
    pub name: String,
    /// The arguments as JSON.
    pub args: String,
}

pub struct JsEngine {
    runtime: Runtime,
    context: Context,
    calls: Rc<RefCell<Vec<HostCall>>>,
    /// When the host last handed control to JavaScript. Read by the watchdog.
    entered: Rc<Cell<Instant>>,
}

impl JsEngine {
    pub fn new() -> Result<Self, String> {
        Self::with_limit(RUNAWAY_SCRIPT)
    }

    fn with_limit(limit: Duration) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
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
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        let start = Instant::now();
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
                        start.elapsed().as_secs_f64() * 1000.0
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
        let result = self.enter().with(|ctx| -> rquickjs::Result<Option<Batch>> {
            let take: Function = entry(&ctx)?.get("take")?;
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
        });
        match result {
            Ok(batch) => batch,
            Err(error) => {
                self.report("taking the op batch", error);
                None
            }
        }
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
        let result = self.enter().with(|ctx| -> rquickjs::Result<()> {
            let timers: Function = entry(&ctx)?.get("timers")?;
            timers.call(())
        });
        if let Err(error) = result {
            self.report("running timers", error);
        }
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

/// How long one stretch of JavaScript may run before it is interrupted.
///
/// The app shares the UI thread with the window: while a script runs, nothing
/// is drawn and no input is read. Ten seconds of that is not slow, it is stuck.
const RUNAWAY_SCRIPT: Duration = Duration::from_secs(10);

/// `globalThis.__sb`, installed by the `@ethercorps/svelte-blitz` package.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_that_never_returns_is_stopped() {
        let js = JsEngine::with_limit(Duration::from_millis(200)).unwrap();
        let started = Instant::now();
        assert!(js.eval("for (;;) {}").is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
        // The engine is still usable, and a later script gets its own time.
        assert_eq!(js.eval_string("String(1 + 1)").unwrap(), "2");
    }
}
