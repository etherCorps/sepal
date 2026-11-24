//! The performance overlay (`src/hud.rs`): nodes the host owns, which the app
//! never sees, and which cost nothing while hidden.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::Path;
use std::sync::{Arc, mpsc};
use std::task::{Context, Wake, Waker};
use std::time::{Duration, Instant};

use blitz_dom::Document;
use sepal::SvelteDocument;
use sepal::app::App;

const COUNTER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/counter/dist");
const HUD: &str = "[data-sepal-hud]";

#[test]
fn the_overlay_is_the_hosts_and_the_app_never_sees_it() {
    let app = App::from_dir(Path::new(COUNTER)).expect("run `pnpm run build` first");
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(800, 600);
    let children = doc.query_all("body > *").len();
    let (live, ops) = (doc.live_nodes(), doc.totals().ops);
    assert!(doc.query(HUD).is_none());

    let began = Instant::now();
    doc.set_hud(true);
    println!("shown in {:?}", began.elapsed());
    doc.resolve();
    let hud = doc.query(HUD).expect("the overlay is on the page");
    let text = doc.text(hud);
    println!("{text}");
    assert!(text.contains("fps"), "{text}");
    if cfg!(unix) {
        assert!(text.contains("MB"), "{text}");
    }
    assert!(text.contains("nodes"), "{text}");

    // Outside <body>, and not one of the app's nodes.
    assert_eq!(doc.query_all("body > *").len(), children);
    assert!(doc.query(&format!("body {HUD}")).is_none());
    assert_eq!(doc.live_nodes(), live);
    assert_eq!(doc.totals().ops, ops, "no op was written for it");

    // In the top-right corner, and clicks go through it.
    let (x, y, width, height) = doc.rect(hud).unwrap();
    println!("at {x},{y} {width}x{height}");
    assert!(x + width > 780.0 && x > 400.0 && y < 20.0, "in the corner");
    let (cx, cy) = ((x + width / 2.0) as f32, (y + height / 2.0) as f32);
    let hit = doc.base().hit(cx, cy).map(|hit| hit.node_id);
    let text_node = doc.base().get_node(hud).unwrap().children[0];
    assert!(
        hit != Some(hud) && hit != Some(text_node),
        "pointer-events: none"
    );

    // The app still works with it up.
    let inc = doc.query("#inc").unwrap();
    doc.click(inc);
    assert_eq!(doc.text(inc), "count is 1");

    #[cfg(feature = "screenshot")]
    {
        let (pixels, w, _) = sepal::screenshot::render_rgba(&mut doc, 800, 600, 2.0);
        // Inside the box, away from the text: the dark background.
        let (px, py) = (
            ((x + width) * 2.0 - 6.0) as u32,
            ((y + height) * 2.0 - 4.0) as u32,
        );
        let at = ((py * w + px) * 4) as usize;
        let [r, g, b] = [pixels[at], pixels[at + 1], pixels[at + 2]];
        assert!(
            r < 60 && g < 60 && b < 60,
            "dark behind the numbers: {r},{g},{b}"
        );
    }

    doc.set_hud(false);
    assert!(doc.query(HUD).is_none(), "hidden means gone");
    assert_eq!(doc.query_all("body > *").len(), children);
    assert!(!doc.poll(None), "and nothing left to do");
}

/// Shown, the overlay wakes the event loop once a second to refresh. Hidden,
/// it does not wake it at all.
#[test]
fn the_overlay_wakes_the_loop_only_while_shown() {
    struct Notify(mpsc::Sender<()>);
    impl Wake for Notify {
        fn wake(self: Arc<Self>) {
            let _ = self.0.send(());
        }
    }
    let (sender, woken) = mpsc::channel();
    let waker = Waker::from(Arc::new(Notify(sender)));

    let app = App::from_dir(Path::new(COUNTER)).unwrap();
    let mut doc = SvelteDocument::for_window(&app).unwrap();
    doc.layout(800, 600);
    doc.poll(Some(Context::from_waker(&waker)));
    assert!(woken.recv_timeout(Duration::from_millis(300)).is_err());

    doc.set_hud(true);
    let text = doc.text(doc.query(HUD).unwrap());
    let started = Instant::now();
    // Something for the "turn" line.
    doc.click(doc.query("#inc").unwrap());
    woken
        .recv_timeout(Duration::from_secs(3))
        .expect("woken to refresh the numbers");
    assert!(started.elapsed() >= Duration::from_millis(900));
    assert!(
        doc.poll(Some(Context::from_waker(&waker))),
        "the refresh asks for a frame"
    );
    assert_eq!(doc.last_turn().ops, 0, "without the app doing anything");
    let refreshed = doc.text(doc.query(HUD).unwrap());
    println!("{refreshed}");
    assert_ne!(refreshed, text, "the first second has been counted");
    assert!(refreshed.contains("ops"), "{refreshed}");

    // To look at.
    #[cfg(feature = "screenshot")]
    {
        let png = sepal::screenshot::render_png(&mut doc, 800, 600, 2.0);
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("hud.png");
        std::fs::write(&out, png).unwrap();
        println!("screenshot: {}", out.display());
    }

    doc.set_hud(false);
    assert!(
        woken.recv_timeout(Duration::from_millis(1500)).is_err(),
        "no wake-ups once hidden"
    );
}
