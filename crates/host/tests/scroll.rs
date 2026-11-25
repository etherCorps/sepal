//! Scroll positions cross the boundary the way layout does: Blitz scrolls a
//! box, the host tells the app at the end of the turn with a `scroll` event,
//! and what the app writes (`scrollTop`, `scrollIntoView()`) goes back as ops.
//! That is what a list needs to render only the rows in view.
//!
//! Uses the shadcn example's bundle for its DOM layer, with a page of its
//! own, and the counter for a plain `onscroll`. Build both first:
//! `pnpm run build` in the repository root.

use std::path::Path;

use sepal::SvelteDocument;

/// A 100px list of 50 rows, 20px each, below a 30px header: row `n` starts at
/// `n * 20` in what the list scrolls over.
const CSS: &str = "
    body { margin: 0 }
    #head { height: 30px }
    #list { height: 100px; width: 200px; overflow-y: scroll }
    #list div { height: 20px }
";

const SETUP: &str = "
    (() => {
        document.body.textContent = '';
        const head = document.createElement('div');
        head.id = 'head';
        document.body.appendChild(head);
        const list = document.createElement('div');
        list.id = 'list';
        for (let n = 0; n < 50; n++) {
            const row = document.createElement('div');
            row.id = 'row' + n;
            row.textContent = 'row ' + n;
            row.addEventListener('click', () => (globalThis.clicked = n));
            list.appendChild(row);
        }
        document.body.appendChild(list);
        globalThis.heard = [];
        list.addEventListener('scroll', (event) => heard.push(event.target.scrollTop));
        return '';
    })()
";

fn page() -> SvelteDocument {
    let mut doc = SvelteDocument::with_css(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/shadcn/dist"
        )),
        CSS,
    )
    .expect("run `pnpm run build` in the repository root first");
    doc.layout(400, 300);
    eval(&mut doc, SETUP);
    doc
}

fn eval(doc: &mut SvelteDocument, code: &str) -> String {
    let result = doc.js().eval_string(code).unwrap();
    doc.settle();
    doc.resolve();
    result
}

/// Which row a click at a point in the list lands on.
fn row_at(doc: &mut SvelteDocument, y: f32) -> String {
    eval(doc, "globalThis.clicked = -1; ''");
    doc.click_at(50.0, y);
    eval(doc, "String(clicked)")
}

#[test]
fn the_wheel_scrolls_a_box_and_the_app_hears_where_to() {
    let mut doc = page();
    assert_eq!(row_at(&mut doc, 40.0), "0", "the first row is at the top");

    doc.wheel(50.0, 80.0, 0.0, 60.0);
    assert_eq!(
        eval(&mut doc, "heard.join()"),
        "60",
        "one scroll event, after the wheel"
    );
    assert_eq!(
        eval(
            &mut doc,
            "const list = document.getElementById('list'); \
             [list.scrollTop, list.scrollHeight, list.clientHeight].join()"
        ),
        "60,1000,100"
    );
    // Rows 3 and on are now where row 0 was.
    assert_eq!(row_at(&mut doc, 40.0), "3");
    // The list's own box does not move when what is in it does.
    assert_eq!(
        eval(
            &mut doc,
            "String(document.getElementById('list').getBoundingClientRect().top)"
        ),
        "30"
    );

    // Past the end, it stops at the end: 1000 - 100.
    doc.wheel(50.0, 80.0, 0.0, 5000.0);
    assert_eq!(eval(&mut doc, "heard.join()"), "60,900");
    // At the end already: nothing moves, nobody is told.
    doc.wheel(50.0, 80.0, 0.0, 50.0);
    assert_eq!(eval(&mut doc, "heard.join()"), "60,900");
}

/// macOS reports a trackpad's travel in points; winit turns that into
/// physical pixels (`PixelDelta`), and blitz-shell 0.3.0-beta.4 passes those
/// to Blitz as CSS pixels. At 2x, 100 points of travel must scroll the box
/// 100 px, not 200.
#[cfg(feature = "window")]
#[test]
fn a_trackpad_scrolls_as_far_as_the_fingers_move_at_2x() {
    use sepal::window::logical_wheel;
    use winit::dpi::LogicalPosition;
    use winit::event::{MouseScrollDelta, TouchPhase, WindowEvent};

    let scale = 2.0;
    let wheel = |delta| WindowEvent::MouseWheel {
        device_id: None,
        delta,
        phase: TouchPhase::Moved,
    };
    // What winit-appkit makes of 100 points of travel that move the content up.
    let from_os = wheel(MouseScrollDelta::PixelDelta(
        LogicalPosition::new(0.0, -100.0).to_physical(scale),
    ));
    let WindowEvent::MouseWheel {
        delta: MouseScrollDelta::PixelDelta(pixels),
        ..
    } = logical_wheel(from_os, scale)
    else {
        panic!("still a pixel delta");
    };
    // blitz-shell hands `pixels.x`, `pixels.y` to Blitz as they are;
    // `wheel` takes them the other way round, as `scrollBy` does.
    let mut doc = page();
    doc.wheel(50.0, 80.0, -pixels.x, -pixels.y);
    assert_eq!(eval(&mut doc, "heard.join()"), "100");

    // Lines have no unit to fix.
    let lines = logical_wheel(wheel(MouseScrollDelta::LineDelta(0.0, -3.0)), scale);
    assert!(matches!(
        lines,
        WindowEvent::MouseWheel {
            delta: MouseScrollDelta::LineDelta(0.0, -3.0),
            ..
        }
    ));
}

#[test]
fn writing_scroll_top_moves_what_is_drawn_and_hit() {
    let mut doc = page();
    // Read back at once, before the host has seen it.
    assert_eq!(
        eval(
            &mut doc,
            "document.getElementById('list').scrollTop = 200; String(document.getElementById('list').scrollTop)"
        ),
        "200"
    );
    assert_eq!(
        eval(&mut doc, "heard.join()"),
        "200",
        "a scroll from the app is heard too"
    );
    assert_eq!(row_at(&mut doc, 40.0), "10");

    // More than it can scroll: the host clamps, and says so.
    eval(
        &mut doc,
        "document.getElementById('list').scrollTop = 99999; ''",
    );
    assert_eq!(eval(&mut doc, "heard.join()"), "200,900");
    assert_eq!(
        eval(
            &mut doc,
            "String(document.getElementById('list').scrollTop)"
        ),
        "900"
    );
    assert_eq!(row_at(&mut doc, 40.0), "45");

    // Rows added and scrolled to in the same turn: the range is the new one.
    eval(
        &mut doc,
        "const list = document.getElementById('list'); \
         for (let n = 50; n < 60; n++) { const row = document.createElement('div'); row.textContent = n; list.appendChild(row); } \
         list.scrollTop = 1100; ''",
    );
    assert_eq!(
        eval(
            &mut doc,
            "String(document.getElementById('list').scrollTop)"
        ),
        "1100"
    );

    // What is drawn moved too: inside the list every pixel row is the one
    // 7px further down before. (Left of the scrollbar, whose thumb moves.)
    #[cfg(feature = "screenshot")]
    {
        use sepal::screenshot::render_rgba;
        eval(
            &mut doc,
            "document.getElementById('list').scrollTop = 0; ''",
        );
        let (top, ..) = render_rgba(&mut doc, 400, 300, 1.0);
        eval(
            &mut doc,
            "document.getElementById('list').scrollTop = 7; ''",
        );
        let (moved, ..) = render_rgba(&mut doc, 400, 300, 1.0);
        let row = |rgba: &[u8], y: usize| rgba[y * 400 * 4..(y * 400 + 150) * 4].to_vec();
        assert_ne!(
            row(&top, 40),
            row(&moved, 40),
            "the list was drawn scrolled"
        );
        for y in 30..123 {
            assert_eq!(row(&moved, y), row(&top, y + 7), "pixel row {y}");
        }
        assert_eq!(
            row(&moved, 140),
            row(&top, 140),
            "below the list nothing moved"
        );
    }
}

#[test]
fn scroll_into_view_scrolls_the_box_around_a_node() {
    let mut doc = page();
    eval(
        &mut doc,
        "document.getElementById('row20').scrollIntoView(); ''",
    );
    assert_eq!(eval(&mut doc, "heard.join()"), "400", "row 20 at the top");
    assert_eq!(row_at(&mut doc, 40.0), "20");

    // Nearest: row 30 is below, so it comes in at the bottom: 30 * 20 + 20 - 100.
    eval(
        &mut doc,
        "document.getElementById('row30').scrollIntoView({ block: 'nearest' }); ''",
    );
    assert_eq!(eval(&mut doc, "heard.join()"), "400,520");
    // Already in view: nothing moves.
    eval(
        &mut doc,
        "document.getElementById('row28').scrollIntoView({ block: 'nearest' }); ''",
    );
    assert_eq!(eval(&mut doc, "heard.join()"), "400,520");
    eval(
        &mut doc,
        "document.getElementById('row1').scrollIntoView({ block: 'nearest' }); ''",
    );
    assert_eq!(eval(&mut doc, "heard.join()"), "400,520,20");
}

/// Svelte's own `onscroll`, in an app without the DOM layer.
#[test]
fn onscroll_in_a_plain_app() {
    let dist = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/counter/dist");
    let app = sepal::app::App::from_dir(Path::new(dist))
        .expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(800, 600);
    let scrolled = doc.query("#scrolled").unwrap();
    assert_eq!(doc.text(scrolled), "scrolled 0px");

    let (x, y, width, height) = doc.rect(doc.query("#rows").unwrap()).unwrap();
    let (x, y) = ((x + width / 2.0) as f32, (y + height / 2.0) as f32);
    doc.wheel(x, y, 0.0, 75.0);
    assert_eq!(doc.text(scrolled), "scrolled 75px");
    assert_eq!(doc.totals().scrolled, 1, "one box, one report");
    doc.wheel(x, y, 0.0, -25.0);
    assert_eq!(doc.text(scrolled), "scrolled 50px");
}
