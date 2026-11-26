//! The embedded JavaScript engine: QuickJS-ng, or V8 with the `engine-v8`
//! feature. Both are a `JsEngine` with the same methods; nothing outside this
//! module knows which one it is talking to.
//!
//! The surface between Rust and JS is deliberately tiny:
//!
//! * JS calls into Rust through `__sb_host` — logging, a clock, and `call`,
//!   which asks the host for something that takes time (see [`HostCall`]).
//! * Rust calls into JS through `__sb` — `dispatch`, `take`, `timers`,
//!   `next_timer`, `frame` and `frame_pending` (`requestAnimationFrame`),
//!   and `resolve` to answer a `call`.
//!
//! Tree mutations never cross this boundary one at a time. They are pulled as
//! one [`Batch`] per turn by [`JsEngine::take_batch`].

use std::time::Duration;

#[cfg(not(feature = "engine-v8"))]
mod quickjs;
#[cfg(not(feature = "engine-v8"))]
pub use quickjs::{JsEngine, compile_folder, engine};

#[cfg(feature = "engine-v8")]
mod v8_engine;
#[cfg(feature = "engine-v8")]
pub use v8_engine::{JsEngine, compile_folder, engine};

/// One turn's worth of tree mutations. See `packages/sepal/src/protocol.js`.
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
    /// `[scrollLeft, scrollTop, scrollWidth, scrollHeight, clientWidth,
    /// clientHeight]` of the box that scrolled. The renderer keeps them.
    Scroll([f64; 6]),
}

/// The property names of [`EventDetail::Scroll`], in order.
pub(crate) const SCROLL_FIELDS: [&str; 6] = [
    "scrollLeft",
    "scrollTop",
    "scrollWidth",
    "scrollHeight",
    "clientWidth",
    "clientHeight",
];

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

/// Where the engines get a module's source, by its URL (`AppFolder::module`).
pub type ModuleSource = Box<dyn Fn(&str) -> Result<String, String>>;

/// Where QuickJS gets a module's bytecode, by its URL, when the app folder
/// has some this host wrote (`bytecode::finder`). V8 compiles JavaScript.
pub type ModuleBytecode = Box<dyn Fn(&str) -> Option<Vec<u8>>>;

/// What `compile_folder` did to an app folder.
#[derive(Debug, Default)]
pub struct Compiled {
    /// Modules compiled and written.
    pub compiled: usize,
    /// Modules that did not compile, with why: they load as JavaScript.
    pub left: Vec<(String, String)>,
    /// Said instead of compiling, by an engine that has no bytecode.
    pub note: Option<&'static str>,
}

/// The URL a module specifier names, from the module or page that imports it
/// (`base`). Modules load only from the app folder; a bare name such as
/// `svelte` means the app was not bundled.
pub(crate) fn resolve_module(base: &str, specifier: &str) -> Result<String, String> {
    use blitz_traits::net::Url;

    let relative = ["/", "./", "../"]
        .iter()
        .any(|start| specifier.starts_with(start));
    let url = if relative {
        Url::parse(base).and_then(|base| base.join(specifier))
    } else {
        Url::parse(specifier)
    }
    .map_err(|_| {
        format!("cannot import {specifier:?}: only paths and URLs of the app's own files can be imported, not package names")
    })?;
    if !crate::net::in_app(&url) {
        return Err(format!(
            "cannot import {url}: modules load only from the app folder"
        ));
    }
    Ok(url.into())
}

/// `import.meta.url` of the module with this name: the name, unless it is
/// an inline module of the page (`…#inline-<node>`), whose URL is the page's.
pub(crate) fn meta_url(name: &str) -> &str {
    name.split_once('#').map_or(name, |(url, _)| url)
}

/// How long one stretch of JavaScript may run before it is interrupted.
///
/// The app shares the UI thread with the window: while a script runs, nothing
/// is drawn and no input is read. Ten seconds of that is not slow, it is stuck.
const RUNAWAY_SCRIPT: Duration = Duration::from_secs(10);

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn a_script_that_never_returns_is_stopped() {
        let js = JsEngine::with_limit(Duration::from_millis(200)).unwrap();
        let started = Instant::now();
        assert!(js.eval("for (;;) {}").is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
        // The engine is still usable, and a later script gets its own time.
        assert_eq!(js.eval_string("String(1 + 1)").unwrap(), "2");
    }

    #[test]
    fn calls_nest_deeply_and_running_out_of_stack_is_an_error() {
        let test = || {
            let js = JsEngine::new().unwrap();
            let depth: u32 = js
                .eval_string(
                    "(() => { let n = 0; const f = () => { n++; f(); }; \
                     try { f(); } catch { return String(n); } })()",
                )
                .unwrap()
                .parse()
                .unwrap();
            println!("calls nest {depth} deep");
            assert!(depth >= 2000, "calls only nest {depth} deep");
            assert_eq!(js.eval_string("String(1 + 1)").unwrap(), "2");
        };
        // On a thread with the stack the UI thread has, whatever the test
        // runner gives its own.
        std::thread::Builder::new()
            .stack_size(8 << 20)
            .spawn(test)
            .unwrap()
            .join()
            .unwrap();
    }
}
