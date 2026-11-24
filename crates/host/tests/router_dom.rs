//! The file router and the DOM layer together: `examples/router-dom-probe`,
//! two routes under one layout, each with a Bits UI Select, Dialog and
//! Tooltip. Bits UI portals its popups into `<body>`, outside the route that
//! made them, so this checks they go when the route goes, and that the
//! router still hears link clicks with the DOM layer loaded.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::Path;

use sepal::SvelteDocument;
use sepal::app::App;

fn load() -> SvelteDocument {
    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/router-dom-probe/dist"
    ));
    let app = App::from_dir(dist).expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(900, 700);
    doc
}

#[track_caller]
fn text(doc: &SvelteDocument, selector: &str) -> String {
    let node = doc
        .query(selector)
        .unwrap_or_else(|| panic!("nothing matches {selector}"));
    doc.text(node)
}

#[track_caller]
fn click(doc: &mut SvelteDocument, selector: &str) {
    let node = doc
        .query(selector)
        .unwrap_or_else(|| panic!("nothing matches {selector}"));
    doc.click(node);
    doc.settle();
    doc.resolve();
}

/// Click something inside the Dialog. The box is centred with
/// `translate(-50%, -50%)`, which `doc.rect` does not apply and hit testing does.
#[track_caller]
fn click_in_dialog(doc: &mut SvelteDocument, selector: &str) {
    let (_, _, dialog_width, dialog_height) = doc
        .rect(doc.query("[data-slot=dialog-content]").unwrap())
        .unwrap();
    let (x, y, width, height) = doc.rect(doc.query(selector).unwrap()).unwrap();
    doc.click_at(
        (x - dialog_width / 2.0 + width / 2.0) as f32,
        (y - dialog_height / 2.0 + height / 2.0) as f32,
    );
    doc.settle();
    doc.resolve();
}

fn focused(doc: &SvelteDocument) -> String {
    doc.query(":focus")
        .and_then(|node| doc.attr(node, "id").or_else(|| doc.attr(node, "data-slot")))
        .unwrap_or_default()
}

/// Whatever Bits UI left in `<body>` besides the app's own tree.
fn popups(doc: &SvelteDocument) -> usize {
    [
        "[data-select-content]",
        "[role=dialog]",
        "[data-dialog-overlay]",
        "[data-tooltip-content]",
        "[data-bits-floating-content-wrapper]",
    ]
    .iter()
    .map(|selector| doc.query_all(selector).len())
    .sum()
}

/// Open and close everything on the page, the way a user would.
fn use_the_page(doc: &mut SvelteDocument, name: &str) {
    assert_eq!(text(doc, "#title"), format!("Page {name}"));

    // Select: opens with this route's items, Escape closes it, focus stays on the trigger.
    click(doc, "#fruit");
    doc.wait(0.2);
    assert_eq!(doc.query_all("[data-select-content]").len(), 1);
    assert!(
        text(doc, "[data-select-content]").contains(&format!("Apple {name}")),
        "the list belongs to page {name}: {}",
        text(doc, "[data-select-content]")
    );
    doc.press("Escape");
    doc.wait(0.3);
    assert!(
        doc.query("[data-select-content]").is_none(),
        "Select closed"
    );
    assert_eq!(focused(doc), "fruit");

    // Dialog: a click inside it reaches its button, Escape closes it, focus goes back.
    click(doc, "#open-dialog");
    doc.wait(0.3);
    assert_eq!(doc.query_all("[role=dialog]").len(), 1);
    assert_eq!(
        text(doc, "[role=dialog] [data-slot=dialog-title]"),
        format!("Dialog {name}")
    );
    click_in_dialog(doc, "#in-dialog");
    assert_eq!(text(doc, "#in-dialog"), "clicked 1");
    assert!(
        doc.query("[role=dialog]").is_some(),
        "the click did not dismiss it"
    );
    doc.press("Escape");
    doc.wait(0.4);
    assert!(doc.query("[role=dialog]").is_none(), "Dialog closed");
    assert_eq!(focused(doc), "open-dialog");
    assert_eq!(doc.attr(doc.query("body").unwrap(), "style"), None);

    // Tooltip: appears on hover, goes when the pointer leaves.
    let (left, top, width, height) = doc.rect(doc.query("#tip").unwrap()).unwrap();
    doc.move_to(5.0, 690.0);
    doc.move_to((left + width / 2.0) as f32, (top + height / 2.0) as f32);
    doc.wait(0.3);
    assert_eq!(text(doc, "[data-tooltip-content]"), format!("Tip {name}"));
    doc.move_to(5.0, 690.0);
    doc.wait(0.4);
    assert!(
        doc.query("[data-tooltip-content]").is_none(),
        "Tooltip gone"
    );

    assert_eq!(popups(doc), 0, "nothing left open on page {name}");
}

#[test]
fn bits_ui_survives_twenty_navigations() {
    let mut doc = load();
    assert_eq!(text(&doc, "#location"), "/");
    let body_children = doc.query_all("body > *").len();

    let mut live = Vec::new();
    for round in 0..20 {
        let (name, link, path) = if round % 2 == 0 {
            ("B", "#nav-b", "/b")
        } else {
            ("A", "#nav-a", "/")
        };
        // The router's click listener sits on the root; the DOM layer must
        // not keep it from seeing a click on a link.
        click(&mut doc, link);
        doc.wait(0.1);
        assert_eq!(text(&doc, "#location"), path, "round {round}");
        assert_eq!(popups(&doc), 0, "round {round}: nothing from the last page");
        assert_eq!(
            doc.query_all("body > *").len(),
            body_children,
            "round {round}: no portal containers left in <body>"
        );
        use_the_page(&mut doc, name);
        doc.collect_garbage();
        live.push(doc.live_nodes());
    }
    println!("live nodes after each navigation: {live:?}");
    assert!(
        live.iter().all(|&n| n == live[0]),
        "live nodes grow with navigation: {live:?}"
    );
}

/// A link inside portalled content (the Dialog) still navigates, and the
/// Dialog it was in goes with the page.
#[test]
fn a_link_inside_a_dialog_navigates() {
    let mut doc = load();
    click(&mut doc, "#open-dialog");
    doc.wait(0.3);
    click_in_dialog(&mut doc, "#dialog-link");
    doc.wait(0.5);
    assert_eq!(text(&doc, "#location"), "/b");
    assert_eq!(text(&doc, "#title"), "Page B");
    assert_eq!(popups(&doc), 0, "the Dialog went with page A");
    assert_eq!(
        doc.attr(doc.query("body").unwrap(), "style"),
        None,
        "scrolling unlocked"
    );
    // And page B works.
    use_the_page(&mut doc, "B");

    // From the keyboard, as a command palette does: Enter on a focused link
    // in the Dialog. Focus starts on the content; Tab reaches the button, then the link.
    click(&mut doc, "#open-dialog");
    doc.wait(0.3);
    doc.press("Tab");
    doc.press("Tab");
    assert_eq!(focused(&doc), "dialog-link");
    doc.press("Enter");
    doc.wait(0.5);
    assert_eq!(text(&doc, "#location"), "/");
    assert_eq!(popups(&doc), 0, "the Dialog went with page B");
    use_the_page(&mut doc, "A");
}

/// A click on an item in the Select's list picks it: the router's listener
/// on the root sees the click too, and must leave it alone.
#[test]
fn a_click_in_a_select_list_picks() {
    let mut doc = load();
    click(&mut doc, "#nav-b");
    click(&mut doc, "#fruit");
    doc.wait(0.2);
    // The list is placed with a transform, which `doc.rect` does not apply.
    let style = doc
        .attr(
            doc.query("[data-bits-floating-content-wrapper]").unwrap(),
            "style",
        )
        .unwrap();
    let inner = style
        .split_once("translate(")
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0;
    let px = |v: &str| v.trim().trim_end_matches("px").parse::<f64>().unwrap();
    let (x, y) = inner.split_once(',').map(|(x, y)| (px(x), px(y))).unwrap();
    let item = doc
        .rect(doc.query("[data-select-item][data-value=banana]").unwrap())
        .unwrap();
    doc.click_at(
        (x + item.0 + item.2 / 2.0) as f32,
        (y + item.1 + item.3 / 2.0) as f32,
    );
    doc.wait(0.4);
    assert_eq!(text(&doc, "#fruit"), "banana");
    assert_eq!(text(&doc, "#location"), "/b", "no navigation");
    assert_eq!(popups(&doc), 0);
}
