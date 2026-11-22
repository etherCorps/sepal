//! File-based routing, driven the way a user drives it: clicks on links and
//! buttons in `examples/routing`, through real hit testing.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::Path;

use svelte_blitz_host::SvelteDocument;
use svelte_blitz_host::app::App;

fn load() -> SvelteDocument {
    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/routing/dist"
    ));
    let app = App::from_dir(dist).expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::for_app(&app).expect("app failed to start");
    doc.layout(720, 480);
    doc
}

/// The text of the one element matching a selector.
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
}

/// Where the app says it is: the heading of the page, the path, the route id.
#[track_caller]
fn at(doc: &SvelteDocument) -> (String, String, String) {
    (
        text(doc, "#title"),
        text(doc, "#location"),
        text(doc, "#route-id"),
    )
}

fn place(title: &str, path: &str, route: &str) -> (String, String, String) {
    (title.into(), path.into(), route.into())
}

#[test]
fn links_change_the_page_and_layouts_stay() {
    let mut doc = load();
    assert_eq!(at(&doc), place("Home", "/", "/"));
    // Data from the root layout's load.
    assert_eq!(text(&doc, "#app-name"), "Routing example");

    // State in the layout, to see whether the layout survives navigation.
    click(&mut doc, "#layout-clicks");
    click(&mut doc, "#layout-clicks");
    let layout_button = doc.query("#layout-clicks").unwrap();

    click(&mut doc, "#nav-about");
    assert_eq!(at(&doc), place("About", "/about", "/about"));
    assert_eq!(text(&doc, "#layout-clicks"), "layout clicks: 2");
    assert_eq!(
        doc.query("#layout-clicks"),
        Some(layout_button),
        "the layout was not recreated"
    );

    // A relative href resolves against the current page, as in a browser.
    click(&mut doc, "#relative");
    assert_eq!(at(&doc), place("Bellows", "/items/2", "/items/[id]"));
}

#[test]
fn params_load_data_and_query() {
    let mut doc = load();
    click(&mut doc, "#nav-items");
    assert_eq!(at(&doc), place("Items", "/items", "/items"));
    assert_eq!(
        doc.query_all("#items a").len(),
        4,
        "the list came from load"
    );

    click(&mut doc, "#items li:nth-child(1) a");
    assert_eq!(at(&doc), place("Anvil", "/items/1", "/items/[id]"));
    assert_eq!(text(&doc, "#param"), "id 1");
    // `await parent()` gave the page's load the layout's data.
    assert_eq!(text(&doc, "#heading"), "Routing example: Anvil");
    assert_eq!(text(&doc, "#from"), "no query");
    assert_eq!(text(&doc, "#looked-at"), "items seen: 1");

    // Same route, another parameter: the page component is kept and its props change.
    let page_state = doc.query("#looked-at").unwrap();
    click(&mut doc, "#next");
    assert_eq!(at(&doc), place("Bellows", "/items/2", "/items/[id]"));
    assert_eq!(text(&doc, "#looked-at"), "items seen: 2");
    assert_eq!(doc.query("#looked-at"), Some(page_state));

    // goto() from code, with a query string.
    click(&mut doc, "#goto");
    assert_eq!(at(&doc), place("Chisel", "/items/3", "/items/[id]"));
    assert_eq!(text(&doc, "#from"), "button");
}

#[test]
fn groups_rest_parameters_and_redirects() {
    let mut doc = load();

    // `(account)` adds a layout and nothing to the path.
    click(&mut doc, "#nav-settings");
    assert_eq!(
        at(&doc),
        place("Settings", "/settings", "/(account)/settings")
    );
    assert!(
        doc.query("#account #title").is_some(),
        "inside the group layout"
    );

    click(&mut doc, "#nav-docs");
    assert_eq!(
        at(&doc),
        place("Docs", "/docs/guide/install", "/docs/[...path]")
    );
    assert_eq!(text(&doc, "#rest"), "guide/install");
    assert!(
        doc.query("#account").is_none(),
        "the group layout is gone again"
    );

    // /old redirects in its load; the location is where we ended up.
    click(&mut doc, "#nav-old");
    assert_eq!(at(&doc), place("About", "/about", "/about"));
}

#[test]
fn errors_render_inside_the_layout() {
    let mut doc = load();

    // No route matches.
    click(&mut doc, "#nav-missing");
    assert_eq!(at(&doc), place("Error 404", "/nowhere", ""));
    assert_eq!(text(&doc, "#error-message"), "Not found: /nowhere");
    assert_eq!(
        text(&doc, "#app-name"),
        "Routing example",
        "still in the root layout"
    );

    // A route matches and its load calls error(404, …).
    click(&mut doc, "#nav-items");
    click(&mut doc, "#item-unknown");
    assert_eq!(at(&doc), place("Error 404", "/items/99", "/items/[id]"));
    assert_eq!(text(&doc, "#error-message"), "No item 99");

    // And the app carries on.
    click(&mut doc, "#nav-home");
    assert_eq!(at(&doc), place("Home", "/", "/"));
}

#[test]
fn back_and_forward() {
    let mut doc = load();
    click(&mut doc, "#nav-about");
    click(&mut doc, "#nav-items");
    click(&mut doc, "#nav-settings");

    click(&mut doc, "#back");
    assert_eq!(at(&doc).1, "/items");
    click(&mut doc, "#back");
    assert_eq!(at(&doc).1, "/about");
    click(&mut doc, "#forward");
    assert_eq!(at(&doc).1, "/items");

    // Going somewhere new drops what was ahead.
    click(&mut doc, "#nav-home");
    click(&mut doc, "#forward");
    assert_eq!(at(&doc).1, "/");

    // Back past the start does nothing.
    for _ in 0..5 {
        click(&mut doc, "#back");
    }
    assert_eq!(at(&doc), place("Home", "/", "/"));
}

#[test]
fn a_slow_load_keeps_the_old_page_until_it_is_done() {
    let mut doc = load();
    click(&mut doc, "#nav-slow");
    assert_eq!(at(&doc), place("Home", "/", "/"), "still loading");

    doc.wait(0.26);
    assert_eq!(at(&doc), place("Slow", "/slow", "/slow"));
    assert_eq!(text(&doc, "#waited"), "waited 200 ms");

    // A navigation that is overtaken by a newer one never shows.
    click(&mut doc, "#nav-home");
    click(&mut doc, "#nav-slow");
    click(&mut doc, "#nav-about");
    doc.wait(0.26);
    assert_eq!(at(&doc), place("About", "/about", "/about"));
}
