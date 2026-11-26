//! An app loaded from `index.html`: the page parsed into the document, its
//! stylesheets and title, its scripts run in order, and modules loaded from
//! the app folder. The page and its modules are written by hand
//! (`tests/pages/basic`), so nothing here needs a build.

use std::path::Path;

use sepal::SvelteDocument;
use sepal::app::App;

fn open() -> SvelteDocument {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/pages/basic"));
    SvelteDocument::for_app(&App::from_dir(dir).unwrap()).unwrap()
}

#[test]
fn scripts_run_in_order_and_modules_load_from_the_app_folder() {
    let doc = open();
    // Classic scripts, then modules, each in document order. A classic
    // script that throws does not stop the next. The dynamic import() of
    // the first resolves in the microtasks after it, before the next script.
    // The remote classic script and the remote module never run.
    let order = doc.js().eval_string("JSON.stringify(order)").unwrap();
    assert_eq!(
        order,
        r#"["classic 1","dynamic later","classic 3, document is object","main dep http://app.localhost/main.js","inline module http://app.localhost/"]"#
    );
}

/// As in a browser, a classic script that throws is reported and the page
/// goes on: the next classic script runs, and so do the modules.
#[test]
fn a_classic_script_that_throws_does_not_stop_the_next() {
    let doc = open();
    let order = doc.js().eval_string("JSON.stringify(order)").unwrap();
    let ran: Vec<String> = serde_json::from_str(&order).unwrap();
    // The second inline classic script throws; `classic 3` is the one after it.
    let after = ran
        .iter()
        .position(|entry| entry.starts_with("classic 3"))
        .expect("the classic script after the one that throws did not run");
    let before = ran.iter().position(|entry| entry == "classic 1").unwrap();
    assert!(before < after, "{ran:?}");
    assert!(
        ran[after + 1..]
            .iter()
            .any(|entry| entry.starts_with("main ")),
        "the module scripts ran after it: {ran:?}"
    );
}

#[test]
fn current_script_is_the_running_scripts_element() {
    let doc = open();
    // SvelteKit's page takes the element its script sits in.
    let holder = doc.query("#holder").unwrap();
    assert_eq!(
        doc.js().eval_string("String(held.node_id)").unwrap(),
        doc.js_id(holder).unwrap().to_string()
    );
    // The object it got is the one the renderer makes that node from.
    assert_eq!(
        doc.js()
            .eval_string("String(__sb_page.held.get(held.node_id) === held)")
            .unwrap(),
        "true"
    );
    // There was a `document` only while the classic scripts ran: an app
    // without the DOM layer must not find one.
    assert_eq!(
        doc.js().eval_string("typeof document").unwrap(),
        "undefined"
    );
}

#[test]
fn the_page_is_parsed_styled_and_titled() {
    let mut doc = open();
    doc.layout(400, 300);
    assert_eq!(doc.title().as_deref(), Some("A page"));
    // A linked stylesheet from the app folder, and a <style>.
    let linked = doc.query("p.linked").unwrap();
    assert_eq!(doc.rect(linked).unwrap().3, 37.0);
    let inline = doc.query("p.inline").unwrap();
    assert_eq!(doc.rect(inline).unwrap().3, 23.0);
}
