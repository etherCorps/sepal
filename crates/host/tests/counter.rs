//! End-to-end test of the example app, without a window:
//! real Svelte bundle -> QuickJS -> op batches -> blitz-dom -> style + layout,
//! and input going back the other way through Blitz's hit testing.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::Path;
use std::time::Instant;

use blitz_dom::DocumentConfig;
use sepal::SvelteDocument;

fn load() -> SvelteDocument {
    let dist = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/counter/dist");
    let js = std::fs::read_to_string(format!("{dist}/app.js"))
        .expect("run `pnpm run build` in the repository root first");
    let css = std::fs::read_to_string(format!("{dist}/app.css")).unwrap();
    SvelteDocument::new(&js, &css, DocumentConfig::default()).expect("app failed to start")
}

#[test]
fn counter_app() {
    let started = Instant::now();
    let mut doc = load();
    let mount = doc.last_turn();
    println!("mount: {mount:?} in {:?}", started.elapsed());
    assert_eq!(mount.batches, 1, "the whole mount is one batch");

    let started = Instant::now();
    doc.layout(800, 600);
    println!("first style + layout of 1000 rows: {:?}", started.elapsed());

    // --- the tree arrived -------------------------------------------------
    let inc = doc.query("#inc").expect("button exists");
    assert_eq!(doc.text(inc), "count is 0");
    assert_eq!(doc.query_all("#rows > li").len(), 1000);
    assert!(doc.query("#badge").is_none());

    // --- Tailwind's stylesheet was applied ---------------------------------
    // `p-6` on <main> is 24px of padding, so the heading starts at x = 24.
    let (x, y, _, _) = doc.rect(doc.query("h1").unwrap()).unwrap();
    assert_eq!((x, y), (24.0, 24.0), "Tailwind padding applied");
    // `h-screen` + `flex-1` + `overflow-y-scroll`: the list fills what is left of
    // the 600px window instead of growing to fit 1000 rows.
    let (_, list_y, _, list_height) = doc.rect(doc.query("#rows").unwrap()).unwrap();
    assert!(
        list_height > 100.0 && list_y + list_height <= 600.0,
        "flex layout applied"
    );
    // `px-4 py-2` gives the button real size.
    let (_, _, width, height) = doc.rect(inc).unwrap();
    assert!(width > 80.0 && height >= 36.0, "button is {width}x{height}");

    // --- click through real hit testing -------------------------------------
    assert!(doc.is_listening("click"));
    assert!(
        !doc.is_listening("pointermove"),
        "unused events never enter JS"
    );

    let started = Instant::now();
    doc.click(inc);
    println!(
        "click + update + restyle: {:?}, {:?}",
        started.elapsed(),
        doc.last_turn()
    );
    assert_eq!(doc.text(inc), "count is 1");
    assert_eq!(doc.text(doc.query("#doubled").unwrap()), "doubled: 2");

    doc.click(inc);
    doc.click(inc);
    assert_eq!(
        doc.text(doc.query("#badge").expect("{#if} branch rendered")),
        "three or more"
    );

    // --- bubbling and fine-grained updates -----------------------------------
    // Click the label <span> inside the third row; the handler is on the <li>.
    let third = doc.query_all("#rows > li")[2];
    let label = doc.query("#rows > li:nth-child(3) > span").unwrap();
    doc.click(label);
    assert!(doc.attr(third, "class").unwrap().contains("selected"));
    assert_eq!(doc.text(third), "Row 3 1");

    // --- keyed reorder moves nodes instead of recreating them ---------------
    let before = doc.query_all("#rows > li");
    let live = doc.live_nodes();
    doc.click(doc.query("#shuffle").unwrap());
    let after = doc.query_all("#rows > li");
    assert_eq!(after.len(), 1000);
    assert_ne!(before, after, "rows moved");
    let (mut a, mut b) = (before.clone(), after.clone());
    a.sort();
    b.sort();
    assert_eq!(a, b, "same nodes, new order");
    assert_eq!(doc.last_turn().created, 0);
    assert_eq!(doc.live_nodes(), live);
}

#[test]
fn removed_nodes_are_freed() {
    let mut doc = load();
    doc.layout(800, 600);
    let inc = doc.query("#inc").unwrap();

    // Show the {#if} badge, then check that the host is holding its nodes.
    doc.click(inc);
    doc.click(inc);
    doc.click(inc);
    assert!(doc.query("#badge").is_some());
    doc.collect_garbage();
    let with_badge = doc.live_nodes();

    // There is no "hide" in the example, so mount a second app over the first
    // to exercise removal: unmount everything through the renderer's own API.
    let removed = doc
        .js()
        .eval_string("globalThis.__test_unmount ? (__test_unmount(), 'ok') : 'missing'")
        .unwrap();
    assert_eq!(removed, "ok", "example exposes __test_unmount");
    doc.settle();
    doc.resolve();
    assert!(doc.query("#inc").is_none(), "tree was removed");

    let stats = doc.collect_garbage();
    println!(
        "after unmount + gc: {stats:?}, live nodes {} -> {}",
        with_badge,
        doc.live_nodes()
    );
    assert!(
        stats.dropped > 3000,
        "detached nodes were freed on the host"
    );
    assert!(doc.live_nodes() < 20, "only the root is left");
}

/// A promise that is rejected with nobody listening is reported, as a
/// browser's console does. One that is handled, at once or a little later in
/// the same turn, is not.
#[test]
fn unhandled_promise_rejections_are_reported() {
    let mut doc = load();
    let reported = |doc: &SvelteDocument| {
        doc.js()
            .eval_string("String(__sb.stats().rejections)")
            .unwrap()
    };
    let run = |doc: &mut SvelteDocument, code: &str| {
        doc.js().eval_string(code).unwrap();
        doc.settle();
    };
    assert_eq!(reported(&doc), "0");

    run(
        &mut doc,
        "Promise.reject(new Error('handled at once')).catch(() => {}); ''",
    );
    run(
        &mut doc,
        "(async () => { try { await Promise.reject(new Error('awaited')) } catch {} })(); ''",
    );
    run(
        &mut doc,
        "{ const late = Promise.reject(new Error('handled a microtask later')); Promise.resolve().then(() => late.catch(() => {})); ''; }",
    );
    assert_eq!(reported(&doc), "0");

    run(&mut doc, "Promise.reject(new Error('nobody listens')); ''");
    assert_eq!(reported(&doc), "1");
    run(
        &mut doc,
        "(async () => { throw new Error('thrown in an async function') })(); ''",
    );
    assert_eq!(reported(&doc), "2");
}

/// The keyboard needs nothing from the app: Tab moves the focus to the first
/// button, and Enter and Space press it.
#[test]
fn the_keyboard_presses_buttons() {
    let mut doc = load();
    doc.layout(800, 600);
    doc.press("Tab");
    assert_eq!(doc.query(":focus"), doc.query("#inc"));
    doc.press("Enter");
    doc.press(" ");
    let inc = doc.query("#inc").unwrap();
    assert_eq!(doc.text(inc), "count is 2");
    // Enter on something that is not a control does nothing.
    doc.click_at(790.0, 590.0);
    doc.press("Enter");
    assert_eq!(doc.text(inc), "count is 2");
}

#[test]
fn idle_turns_do_no_work() {
    use blitz_dom::Document;

    let mut doc = load();
    doc.layout(800, 600);

    // Nothing happened, so nothing crosses the boundary and no redraw is requested.
    for _ in 0..3 {
        assert!(!doc.poll(None), "poll reports no change while idle");
        assert_eq!(doc.last_turn().ops, 0);
    }
    assert!(!doc.base().has_changes());

    // Moving the mouse over the app does not enter JS at all: nobody listens.
    assert!(!doc.is_listening("pointermove") && !doc.is_listening("mousemove"));
}

#[test]
fn timers_wake_the_event_loop() {
    use blitz_dom::Document;
    use std::sync::{Arc, mpsc};
    use std::task::{Context, Wake, Waker};
    use std::time::Duration;

    struct Notify(mpsc::Sender<()>);
    impl Wake for Notify {
        fn wake(self: Arc<Self>) {
            let _ = self.0.send(());
        }
    }

    let (sender, woken) = mpsc::channel();
    let waker = Waker::from(Arc::new(Notify(sender)));

    // As in a window: only there does time pass by itself. In any other
    // document the clock is the test's to move (`doc.wait`).
    let dist = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/counter/dist");
    let app = sepal::app::App::from_dir(Path::new(dist)).unwrap();
    let mut doc = SvelteDocument::for_window(&app).unwrap();
    doc.layout(800, 600);
    doc.poll(Some(Context::from_waker(&waker)));

    // No timer is running: the timer thread must stay silent.
    assert!(
        woken.recv_timeout(Duration::from_millis(300)).is_err(),
        "no wake-ups while idle"
    );

    // Start the 1s interval from the UI.
    doc.click(doc.query("#tick").unwrap());
    assert_eq!(doc.text(doc.query("#seconds").unwrap()), "0s");

    // The host is woken once, about a second later, and the tick lands.
    let started = Instant::now();
    woken
        .recv_timeout(Duration::from_secs(3))
        .expect("woken for the interval");
    let waited = started.elapsed();
    assert!(
        waited >= Duration::from_millis(900),
        "woke too early: {waited:?}"
    );
    assert!(
        doc.poll(Some(Context::from_waker(&waker))),
        "the tick changed the tree"
    );
    assert_eq!(doc.last_turn().ops, 1, "one text node changed");
    assert_eq!(doc.text(doc.query("#seconds").unwrap()), "1s");

    // Stop it again: no further wake-ups.
    doc.click(doc.query("#tick").unwrap());
    assert!(
        woken.recv_timeout(Duration::from_millis(1500)).is_err(),
        "timer stopped"
    );
}

#[cfg(feature = "screenshot")]
#[test]
fn screenshot_after_interaction() {
    use sepal::screenshot::render_png;

    let mut doc = load();
    doc.layout(800, 600);
    let inc = doc.query("#inc").unwrap();
    for _ in 0..3 {
        doc.click(inc);
    }
    doc.click(doc.query("#rows > li:nth-child(3)").unwrap());

    let png = render_png(&mut doc, 800, 600, 1.0);
    assert!(png.len() > 10_000, "rendered something");
    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../../screenshot.png");
    std::fs::write(out, png).unwrap();
}

/// What a dev reload does: build a second document while the first is alive,
/// then drop the first. Each owns its own JS engine and timer thread.
#[test]
fn a_fresh_document_can_replace_a_running_one() {
    let mut old = load();
    old.layout(800, 600);
    old.click(old.query("#inc").unwrap());
    old.click(old.query("#tick").unwrap()); // leave an interval running

    let mut new = load();
    drop(old);
    new.layout(800, 600);

    let inc = new.query("#inc").unwrap();
    assert_eq!(new.text(inc), "count is 0", "state starts over");
    new.click(inc);
    assert_eq!(new.text(inc), "count is 1");
    assert_eq!(new.text(new.query("#seconds").unwrap()), "0s");
}
