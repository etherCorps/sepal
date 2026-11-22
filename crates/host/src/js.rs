//! The embedded JavaScript engine (QuickJS-ng through `rquickjs`).
//!
//! The surface between Rust and JS is deliberately tiny:
//!
//! * JS calls into Rust through `__sb_host` — logging and a clock.
//! * Rust calls into JS through `__sb` — `dispatch`, `take`, `timers` and `next_timer`.
//!
//! Tree mutations never cross this boundary one at a time. They are pulled as
//! one [`Batch`] per turn by [`JsEngine::take_batch`].

use std::time::Instant;

use rquickjs::{CatchResultExt, Context, Ctx, Function, Object, Runtime, TypedArray, Value};

/// One turn's worth of tree mutations. See `packages/renderer/src/protocol.js`.
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

pub struct JsEngine {
    runtime: Runtime,
    context: Context,
}

impl JsEngine {
    pub fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        let start = Instant::now();

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

                ctx.globals().set("__sb_host", host)?;
                Ok(())
            })
            .map_err(|e| e.to_string())?;

        Ok(Self { runtime, context })
    }

    /// Evaluate a script (the app bundle) and run the microtasks it queued.
    pub fn eval(&self, code: &str) -> Result<(), String> {
        self.context.with(|ctx| {
            ctx.eval::<(), _>(code)
                .catch(&ctx)
                .map_err(|e| e.to_string())
        })?;
        self.run_jobs();
        Ok(())
    }

    /// Evaluate an expression and return its string value. Handy in tests.
    pub fn eval_string(&self, code: &str) -> Result<String, String> {
        let out = self.context.with(|ctx| {
            ctx.eval::<String, _>(code)
                .catch(&ctx)
                .map_err(|e| e.to_string())
        })?;
        self.run_jobs();
        Ok(out)
    }

    /// Run promise reactions and other queued jobs until the queue is empty.
    pub fn run_jobs(&self) {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(true) => continue,
                Ok(false) => break,
                Err(error) => {
                    eprintln!("[js error] uncaught in job: {error:?}");
                }
            }
        }
    }

    /// Deliver a DOM event to the renderer's dispatcher. Returns `DISPATCH_*` flags.
    pub fn dispatch(&self, target: u32, name: &str, bubbles: bool, detail: &EventDetail) -> u32 {
        let result = self.context.with(|ctx| -> rquickjs::Result<u32> {
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
        let result = self.context.with(|ctx| -> rquickjs::Result<Option<Batch>> {
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

    /// Run the timers that are due.
    pub fn run_timers(&self) {
        let result = self.context.with(|ctx| -> rquickjs::Result<()> {
            let timers: Function = entry(&ctx)?.get("timers")?;
            timers.call(())
        });
        if let Err(error) = result {
            self.report("running timers", error);
        }
    }

    /// The next timer deadline in host-clock milliseconds, if any timer is pending.
    pub fn next_timer(&self) -> Option<f64> {
        let result = self.context.with(|ctx| -> rquickjs::Result<f64> {
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
        let message = self.context.with(|ctx| {
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
