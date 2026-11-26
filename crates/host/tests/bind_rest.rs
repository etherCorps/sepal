//! `bind:` through `{...rest}` props, which depends on `shims.js`'s fix for
//! QuickJS's accessor descriptors through a Proxy. Svelte's runtime keeps its
//! own copy of `Object.getOwnPropertyDescriptor` when it loads, so the fix has
//! to be in place first. `examples/bind-rest` imports its component before
//! Sepal; `sepal()` puts Sepal's runtime first in the page's modules anyway.
//! V8 has no such bug, so on V8 this passes either way.

use std::path::Path;

use sepal::SvelteDocument;
use sepal::app::App;

#[test]
fn bind_through_rest_props_reaches_the_parent() {
    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/bind-rest/dist"
    ));
    let app = App::from_dir(dist).expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(400, 300);
    let out = doc.query("#out").unwrap();
    assert_eq!(doc.text(out), "start");

    doc.click(doc.query("#change").unwrap());
    doc.settle();
    assert_eq!(doc.text(doc.query("#out").unwrap()), "changed");
}
