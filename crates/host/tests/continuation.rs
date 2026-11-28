//! The value that follows the code through promise jobs (`__sb_host.continuation`
//! and `set_continuation`): QuickJS-ng #1920, patched into the vendored
//! rquickjs-sys, and V8's continuation-preserved embedder data. One test for
//! each rule #1920 defines, then the ones the runtime adds (timers, frame
//! callbacks, host calls) and `AsyncLocalStorage` (`async-context.js`) on top.
//!
//! The page is a copy of the built counter app, for its runtime (`shims.js`,
//! `fetch`), with `async-context.js` copied next to it.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sepal::SvelteDocument;
use sepal::app::App;

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn open(name: &str) -> SvelteDocument {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir: PathBuf =
        std::env::temp_dir().join(format!("sepal-continuation-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy_dir(&root.join("../../examples/counter/dist"), &dir);
    fs::copy(
        root.join("../../packages/sepal/src/async-context.js"),
        dir.join("async-context.js"),
    )
    .unwrap();
    let app = App::from_dir(&dir).expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(400, 300);
    doc
}

/// Run `script`, which fills `globalThis.seen` and sets `globalThis.done`,
/// letting virtual time pass and host calls finish. Returns `seen` as JSON.
fn run(doc: &mut SvelteDocument, script: &str) -> serde_json::Value {
    let wrapped = format!(
        "globalThis.seen = {{}}; globalThis.done = false;
         const get = () => __sb_host.continuation();
         const set = (value) => __sb_host.set_continuation(value);
         {script}
         return 'started';"
    );
    doc.js()
        .eval_string(&format!("(() => {{ {wrapped} }})()"))
        .unwrap();
    let started = Instant::now();
    while doc.js().eval_string("String(globalThis.done)").unwrap() != "true" {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{script} never finished"
        );
        doc.wait(0.02);
        std::thread::sleep(Duration::from_millis(2));
    }
    serde_json::from_str(
        &doc.js()
            .eval_string("JSON.stringify(globalThis.seen)")
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn a_value_set_outside_a_job_stays_until_set_again() {
    let mut doc = open("stays");
    let seen = run(
        &mut doc,
        "seen.initial = get() === undefined;
         set('a'); seen.set = get();
         set(undefined); seen.cleared = get() === undefined;
         done = true;",
    );
    assert_eq!(seen["initial"], true);
    assert_eq!(seen["set"], "a");
    assert_eq!(seen["cleared"], true);
    // And across turns: the host does not reset it.
    doc.js()
        .eval_string("__sb_host.set_continuation('kept'); 'ok'")
        .unwrap();
    doc.wait(0.05);
    assert_eq!(
        doc.js()
            .eval_string("String(__sb_host.continuation())")
            .unwrap(),
        "kept"
    );
}

#[test]
fn then_and_await_run_with_the_value_of_where_they_were_registered() {
    let mut doc = open("then");
    let seen = run(
        &mut doc,
        "let resolve;
         const promise = new Promise((r) => (resolve = r));
         set('then'); promise.then(() => (seen.then = get()));
         set('await'); (async () => { await promise; seen.await = get(); })();
         set('settles'); resolve();
         set(undefined);
         promise.then(() => setTimeout(() => (done = true)));",
    );
    assert_eq!(
        seen["then"], "then",
        "not the value of the code that settles it"
    );
    assert_eq!(seen["await"], "await");
}

#[test]
fn a_reaction_to_a_settled_promise_and_a_microtask_run_with_the_value_of_where_they_were_queued() {
    let mut doc = open("queued");
    let seen = run(
        &mut doc,
        "set('settled'); Promise.resolve().then(() => (seen.settled = get()));
         set('microtask'); queueMicrotask(() => (seen.microtask = get()));
         set('chain');
         Promise.resolve()
             .then(() => { seen.first = get(); set('changed inside'); })
             .then(() => { seen.second = get(); done = true; });
         set(undefined);",
    );
    assert_eq!(seen["settled"], "settled");
    assert_eq!(seen["microtask"], "microtask");
    assert_eq!(seen["first"], "chain");
    // Registered while 'chain' was current: what the first reaction set
    // inside itself does not leak into the next.
    assert_eq!(seen["second"], "chain");
}

#[test]
fn the_previous_value_is_restored_when_a_job_ends() {
    let mut doc = open("restored");
    let seen = run(
        &mut doc,
        "set('job');
         Promise.resolve().then(() => { seen.inside = get(); set('changed inside the job'); });
         set('outside');
         seen.after_queueing = get();
         set(undefined);
         setTimeout(() => { done = true; });",
    );
    assert_eq!(seen["inside"], "job");
    assert_eq!(seen["after_queueing"], "outside");
    // Back at the top of a turn, after the job that changed it ran: the
    // value from before the job, not what the job set.
    assert_eq!(
        doc.js()
            .eval_string("String(__sb_host.continuation())")
            .unwrap(),
        "undefined"
    );
}

#[test]
fn an_awaited_thenable_runs_with_the_value_of_the_code_that_resolved_with_it() {
    let mut doc = open("thenable");
    let seen = run(
        &mut doc,
        "const thenable = { then(resolve) { seen.then = get(); resolve(1); } };
         set('resolver'); Promise.resolve(thenable).then(() => { done = true; });
         set(undefined);",
    );
    assert_eq!(seen["then"], "resolver");
}

#[cfg(not(feature = "engine-v8"))]
#[test]
fn a_finalization_registry_callback_runs_without_a_value() {
    let mut doc = open("registry");
    doc.js()
        .eval_string(
            "globalThis.seen = {};
             globalThis.registry = new FinalizationRegistry(() => {
                 seen.registry = __sb_host.continuation() === undefined;
             });
             __sb_host.set_continuation('registered');
             (() => { registry.register({}, 'held'); })();
             __sb_host.set_continuation('collecting');
             'ok'",
        )
        .unwrap();
    doc.collect_garbage();
    doc.wait(0.05);
    assert_eq!(
        doc.js().eval_string("String(seen.registry)").unwrap(),
        "true",
        "neither the value of where it was registered nor of the code that freed the object"
    );
}

#[test]
fn timers_and_frame_callbacks_run_with_the_value_of_where_they_were_scheduled() {
    let mut doc = open("timers");
    let seen = run(
        &mut doc,
        "set('timeout'); setTimeout(() => { seen.timeout = get(); });
         set('interval');
         let ticks = 0;
         const id = setInterval(() => {
             (seen.interval ??= []).push(get());
             if (++ticks === 2) clearInterval(id);
         }, 5);
         set('frame'); requestAnimationFrame(() => { seen.frame = get(); });
         set(undefined);
         setTimeout(() => { seen.after = get() === undefined; done = true; }, 50);",
    );
    assert_eq!(seen["timeout"], "timeout");
    assert_eq!(
        seen["interval"],
        serde_json::json!(["interval", "interval"])
    );
    assert_eq!(
        seen["after"], true,
        "a timer scheduled without a value runs without one"
    );
    // Headless, frames come with the virtual time's steps (`doc.wait`).
    assert_eq!(seen["frame"], "frame");
}

#[test]
fn code_awaiting_a_host_call_resumes_with_its_value() {
    let mut doc = open("host-call");
    let seen = run(
        &mut doc,
        "set('fetching');
         (async () => {
             const response = await fetch('/index.html');
             seen.fetched = get();
             await response.text();
             seen.read = get();
             done = true;
         })();
         set(undefined);",
    );
    assert_eq!(seen["fetched"], "fetching");
    assert_eq!(seen["read"], "fetching");
}

#[test]
fn async_local_storage_follows_each_run_through_awaits_timers_and_host_calls() {
    let mut doc = open("storage");
    let seen = run(
        &mut doc,
        "import('http://app.localhost/async-context.js').then(async ({ AsyncLocalStorage }) => {
             const storage = new AsyncLocalStorage();
             const other = new AsyncLocalStorage();
             const tick = () => new Promise((resolve) => setTimeout(resolve, 5));
             seen.outside = storage.getStore() === undefined;
             // Two runs at once, each awaiting the other's turn.
             const run = (name) => storage.run(name, async () => {
                 await tick();
                 const first = storage.getStore();
                 await fetch('/index.html');
                 const second = storage.getStore();
                 const nested = await other.run('nested ' + name, async () => {
                     await tick();
                     return [storage.getStore(), other.getStore()];
                 });
                 const timer = await new Promise((resolve) => setTimeout(() => resolve(storage.getStore())));
                 return { first, second, nested, after_nested: other.getStore() === undefined, timer };
             });
             const [a, b] = await Promise.all([run('a'), run('b')]);
             seen.a = a; seen.b = b;
             seen.exit = storage.run('x', () => storage.exit(() => storage.getStore() === undefined));
             const bound = storage.run('bound', () => AsyncLocalStorage.bind(() => storage.getStore()));
             seen.bound = bound();
             const snapshot = storage.run('snapshot', () => AsyncLocalStorage.snapshot());
             seen.snapshot = snapshot(() => storage.getStore());
             await storage.run('entered', async () => {
                 await tick();
                 storage.enterWith('changed');
                 seen.entered = storage.getStore();
             });
             seen.after = storage.getStore() === undefined;
             done = true;
         });",
    );
    assert_eq!(seen["outside"], true);
    for name in ["a", "b"] {
        let run = &seen[name];
        assert_eq!(run["first"], name, "{seen}");
        assert_eq!(run["second"], name, "after a host call: {seen}");
        assert_eq!(
            run["nested"],
            serde_json::json!([name, format!("nested {name}")])
        );
        assert_eq!(run["after_nested"], true);
        assert_eq!(run["timer"], name);
    }
    assert_eq!(seen["exit"], true);
    assert_eq!(seen["bound"], "bound");
    assert_eq!(seen["snapshot"], "snapshot");
    assert_eq!(seen["entered"], "changed");
    assert_eq!(seen["after"], true, "a run's values end with it");
}
