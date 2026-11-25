//! `requestAnimationFrame`: callbacks run when the host draws a frame,
//! before it lays out and paints, and nothing asks for a frame while none
//! is queued (invariant 7). And the timers that follow frames: a
//! `setTimeout(0)` runs in the next turn, without waiting for the timer
//! thread, and a chain of them is held to HTML's 4 ms.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::Path;
use std::sync::{Arc, mpsc};
use std::task::{Context, Wake, Waker};
use std::time::{Duration, Instant};

use blitz_dom::Document;
use sepal::SvelteDocument;

const COUNTER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/counter/dist");

fn load() -> SvelteDocument {
    let app = sepal::app::App::from_dir(Path::new(COUNTER))
        .expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(800, 600);
    doc
}

fn run(doc: &SvelteDocument, code: &str) -> String {
    doc.js().eval_string(code).unwrap()
}

#[test]
fn callbacks_run_in_order_with_the_frames_time() {
    let mut doc = load();
    doc.wait(0.1);
    run(
        &doc,
        "globalThis.log = [];
         requestAnimationFrame((t) => log.push('a ' + Math.round(t) + ' ' + (t === performance.now())));
         const b = requestAnimationFrame(() => log.push('b'));
         requestAnimationFrame(() => { log.push('c'); cancelAnimationFrame(d); });
         const d = requestAnimationFrame(() => log.push('d'));
         requestAnimationFrame(() => log.push('e'));
         cancelAnimationFrame(b);
         ''",
    );
    // Nothing runs until a frame is drawn, and the turn asks for one.
    assert!(doc.settle().frame, "a queued callback asks for a frame");
    assert_eq!(run(&doc, "log.join()"), "");
    doc.begin_frame();
    assert_eq!(
        run(&doc, "log.join()"),
        "a 100 true,c,e",
        "in order, with the frame's time, which is performance.now()'s; \
         cancelled ones, also from an earlier callback, do not run"
    );
    assert!(!doc.settle().frame, "no frame asked for once they ran");
}

#[test]
fn a_callback_queued_in_a_frame_runs_in_the_next() {
    let mut doc = load();
    run(
        &doc,
        "globalThis.log = [];
         requestAnimationFrame((t) => { log.push('first ' + Math.round(t)); requestAnimationFrame((t) => log.push('second ' + Math.round(t))); });
         ''",
    );
    doc.wait(0.02);
    assert_eq!(run(&doc, "log.join()"), "first 20");
    assert!(doc.last_turn().frame, "the next frame is asked for");
    doc.wait(0.02);
    assert_eq!(run(&doc, "log.join()"), "first 20,second 40");
}

#[test]
fn a_timeout_from_a_frame_waits_for_the_turn_after_it() {
    let mut doc = load();
    run(
        &doc,
        "globalThis.log = [];
         requestAnimationFrame(() => { log.push('frame'); setTimeout(() => log.push('timeout'), 0); });
         ''",
    );
    doc.begin_frame();
    assert_eq!(run(&doc, "log.join()"), "frame", "the frame paints first");
    doc.settle();
    assert_eq!(
        run(&doc, "log.join()"),
        "frame,timeout",
        "then the next turn runs it"
    );
}

#[test]
fn a_hidden_window_draws_no_frames() {
    let mut doc = load();
    doc.set_visible(false);
    run(
        &doc,
        "globalThis.log = []; requestAnimationFrame((t) => log.push('ran ' + Math.round(t))); ''",
    );
    assert!(!doc.settle().frame, "hidden, no frame is asked for");
    doc.wait(1.0);
    assert_eq!(run(&doc, "log.join()"), "", "the callback waits");

    doc.set_visible(true);
    assert!(doc.settle().frame, "shown again, it asks for one");
    doc.wait(0.02);
    assert_eq!(
        run(&doc, "log.join()"),
        "ran 1020",
        "and runs on the first frame"
    );
}

/// In a window time is real: the frame's timestamp is taken as it starts,
/// and `performance.now()` inside a callback is on the same clock, a little later.
#[test]
fn in_a_window_the_timestamp_is_performance_now() {
    let app = sepal::app::App::from_dir(Path::new(COUNTER)).unwrap();
    let mut doc = SvelteDocument::for_window(&app).unwrap();
    doc.layout(800, 600);
    run(
        &doc,
        "globalThis.gap = null; requestAnimationFrame((t) => { gap = performance.now() - t; }); ''",
    );
    std::thread::sleep(std::time::Duration::from_millis(50));
    doc.begin_frame();
    let gap: f64 = run(&doc, "String(gap)").parse().unwrap();
    assert!(
        (0.0..5.0).contains(&gap),
        "performance.now() is {gap} ms after the frame's timestamp"
    );
}

/// A document as in a window, on real time, and the wake-ups it asks for.
fn windowed() -> (SvelteDocument, Waker, mpsc::Receiver<()>) {
    struct Notify(mpsc::Sender<()>);
    impl Wake for Notify {
        fn wake(self: Arc<Self>) {
            let _ = self.0.send(());
        }
    }
    let (sender, woken) = mpsc::channel();
    let waker = Waker::from(Arc::new(Notify(sender)));
    let app = sepal::app::App::from_dir(Path::new(COUNTER)).unwrap();
    let mut doc = SvelteDocument::for_window(&app).unwrap();
    doc.layout(800, 600);
    doc.poll(Some(Context::from_waker(&waker)));
    (doc, waker, woken)
}

#[test]
fn a_timeout_from_a_frame_wakes_the_loop_at_once() {
    let (mut doc, waker, woken) = windowed();
    run(
        &doc,
        "globalThis.log = [];
         requestAnimationFrame(() => setTimeout(() => log.push('timeout'), 0));
         ''",
    );
    assert!(
        doc.poll(Some(Context::from_waker(&waker))),
        "asks for a frame"
    );
    while woken.try_recv().is_ok() {}

    doc.begin_frame();
    // Woken before `begin_frame` returned: from the UI thread, not the timer thread.
    assert!(woken.try_recv().is_ok(), "the due timer woke the loop");
    assert_eq!(run(&doc, "log.join()"), "");
    assert!(!doc.poll(Some(Context::from_waker(&waker))));
    assert_eq!(run(&doc, "log.join()"), "timeout", "the next turn ran it");
    assert!(
        woken.recv_timeout(Duration::from_millis(300)).is_err(),
        "then the loop sleeps"
    );
}

#[test]
fn a_chain_of_zero_timeouts_wakes_the_loop_at_most_250_times_a_second() {
    let (mut doc, waker, woken) = windowed();
    run(
        &doc,
        "globalThis.n = 0;
         globalThis.tick = () => { n++; if (globalThis.tick) setTimeout(globalThis.tick, 0); };
         setTimeout(tick, 0);
         ''",
    );
    doc.poll(Some(Context::from_waker(&waker)));
    let started = Instant::now();
    let mut wakes = 0;
    while started.elapsed() < Duration::from_secs(1) {
        if woken.recv_timeout(Duration::from_millis(100)).is_ok() {
            wakes += 1;
            doc.poll(Some(Context::from_waker(&waker)));
        }
    }
    run(&doc, "globalThis.tick = null; ''");
    let ran: u32 = run(&doc, "String(n)").parse().unwrap();
    println!("{wakes} wake-ups and {ran} callbacks in a second");
    // The ceiling is ours, the floor the OS's: macOS coalesces the timers of a
    // process it runs at a lower priority. Here, from a terminal in front,
    // ~206; under `taskpolicy -c utility` ~70, under `taskpolicy -b` ~20.
    // Ten says only that the chain keeps going.
    assert!(
        (10..=300).contains(&wakes),
        "{wakes} wake-ups in a second: from the sixth nested timer on, each waits 4 ms"
    );
}
