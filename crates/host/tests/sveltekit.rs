//! SvelteKit itself, built as a single-page app (`examples/sveltekit`), with
//! its own client router running in the host.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::Path;

use svelte_blitz_host::SvelteDocument;
use svelte_blitz_host::app::App;

fn load() -> SvelteDocument {
    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sveltekit/dist"
    ));
    let app = App::from_dir(dist).expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(800, 600);
    // SvelteKit loads the first route's modules before it shows anything.
    doc.wait(0.1);
    doc
}

fn text(doc: &SvelteDocument, selector: &str) -> String {
    doc.query(selector)
        .map(|node| doc.text(node))
        .unwrap_or_else(|| format!("(no {selector})"))
}

fn click(doc: &mut SvelteDocument, selector: &str) {
    let node = doc
        .query(selector)
        .unwrap_or_else(|| panic!("nothing matches {selector}"));
    doc.click(node);
    doc.wait(0.1);
}

#[test]
fn sveltekit_routes_between_pages() {
    let mut doc = load();
    assert_eq!(text(&doc, "#title"), "Home");
    assert_eq!(text(&doc, "#path"), "/");

    // The page is alive: a component's own state.
    click(&mut doc, "#count");
    assert_eq!(text(&doc, "#count"), "clicked 1");

    // A link. SvelteKit hears the click on <html>, reads the link's address
    // and changes the page without loading anything.
    click(&mut doc, "#about");
    assert_eq!(text(&doc, "#title"), "About");
    assert_eq!(
        text(&doc, "#path"),
        "/about",
        "`page` from $app/state follows"
    );

    // `goto`, to a route with a parameter and a `load` function.
    click(&mut doc, "#go");
    assert_eq!(text(&doc, "#title"), "Item 7");
    assert_eq!(text(&doc, "#double"), "twice that is 14");

    // The same route with another parameter: `load` runs again.
    click(&mut doc, "#item");
    assert_eq!(text(&doc, "#title"), "Item 42");
    assert_eq!(text(&doc, "#double"), "twice that is 84");

    // Back, twice, through `history`.
    let back = |doc: &mut SvelteDocument| {
        doc.js().eval_string("history.back(); ''").unwrap();
        doc.wait(0.1);
    };
    back(&mut doc);
    assert_eq!(text(&doc, "#title"), "Item 7");
    back(&mut doc);
    assert_eq!(text(&doc, "#title"), "About");
    assert_eq!(
        doc.js()
            .eval_string("location.pathname + ' ' + history.length")
            .unwrap(),
        "/about 4"
    );
}
