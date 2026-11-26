//! SvelteKit itself (`examples/sveltekit`: `sveltekit(withSepal(…))`,
//! `sepal/adapter`, `sepal()`), SvelteKit's split build loaded from its
//! index.html, with its own client router running in the host.
//!
//! Build the app first: `pnpm run build` in the repository root.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sepal::SvelteDocument;
use sepal::api::Permissions;
use sepal::app::App;

fn dist() -> PathBuf {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sveltekit/dist"
    ))
    .to_path_buf()
}

fn load() -> SvelteDocument {
    load_with(None)
}

/// With these permissions instead of the app's own.
fn load_with(permissions: Option<Permissions>) -> SvelteDocument {
    let mut app =
        App::from_dir(&dist()).expect("run `pnpm run build` in the repository root first");
    if let Some(permissions) = permissions {
        app.manifest.permissions = permissions;
    }
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

    // The layout's <style> made it into app.css: the bar is a flex row.
    let (_, _, _, bar) = doc.rect(doc.query("nav").unwrap()).unwrap();
    assert_eq!(bar, 48.0);
    // A static file, written the way SvelteKit apps write it: from the root.
    let logo = doc.query("#logo").unwrap();
    assert_eq!(doc.attr(logo, "src").as_deref(), Some("/logo.svg"));
    let (pixels, width, _) = sepal::screenshot::render_rgba(&mut doc, 800, 600, 1.0);
    let (x, y, w, h) = doc.rect(logo).unwrap();
    let at = ((y + h / 2.0) as usize * width as usize + (x + w / 2.0) as usize) * 4;
    // The bar behind it is dark, so white here is the mark itself.
    assert!(
        pixels[at..at + 3].iter().all(|&channel| channel > 200),
        "the logo is drawn, in white: {:?}",
        &pixels[at..at + 3]
    );

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
    // `<svelte:head><title>`, which Svelte refuses for a custom renderer and
    // the build rewrites.
    let title = |doc: &SvelteDocument| doc.js().eval_string("document.title").unwrap();
    assert_eq!(title(&doc), "About this app");

    // `goto`, to a route with a parameter and a `load` function.
    click(&mut doc, "#go");
    assert_eq!(text(&doc, "#title"), "Item 7");
    assert_eq!(text(&doc, "#double"), "twice that is 14");
    assert_eq!(title(&doc), "Item 7", "a title with an expression in it");

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

/// A stylesheet the app adds at run time gets `load` or `error`, as in a
/// browser: Vite waits for one before it runs a chunk with styles of its
/// own. It sets `rel` as a property, so that is what this does. A missing
/// file and one on the network are errors; the host never fetches the second.
#[test]
fn stylesheets_added_at_run_time_say_whether_they_loaded() {
    let mut doc = load();
    let assets = dist().join("_app/immutable/assets");
    let css = std::fs::read_dir(&assets)
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .find(|name| name.ends_with(".css"))
        .expect("SvelteKit wrote a stylesheet");
    let script = format!(
        "globalThis.heard = [];
         for (const href of ['/_app/immutable/assets/{css}', '/missing.css', 'https://example.com/remote.css']) {{
             const link = document.createElement('link');
             link.rel = 'stylesheet';
             link.href = href;
             link.addEventListener('load', () => heard.push('load ' + href));
             link.addEventListener('error', () => heard.push('error ' + href));
             document.head.appendChild(link);
         }}
         'ok'"
    );
    doc.js().eval_string(&script).unwrap();
    doc.settle();
    assert_eq!(
        doc.js().eval_string("heard.join(', ')").unwrap(),
        format!(
            "load /_app/immutable/assets/{css}, error /missing.css, error https://example.com/remote.css"
        )
    );
}

/// Modules load from the app folder only: an import from the network is
/// refused, and the app hears why.
#[test]
fn a_module_from_the_network_is_refused() {
    let mut doc = load();
    doc.js()
        .eval_string(
            "globalThis.outcome = 'pending';
             import('https://example.com/remote.js').then(() => (outcome = 'loaded'), (error) => (outcome = 'refused: ' + error.message));
             'ok'",
        )
        .unwrap();
    doc.settle();
    let outcome = doc.js().eval_string("outcome").unwrap();
    assert!(outcome.starts_with("refused: "), "{outcome}");
}

/// `document.title` (and `<svelte:head><title>`) reaches the window without
/// `permissions.window`, as a page's title does on the web.
#[test]
fn the_title_reaches_the_window_without_permission() {
    #[derive(Default)]
    struct Titles(Mutex<Vec<String>>);
    impl blitz_traits::shell::ShellProvider for Titles {
        fn set_window_title(&self, title: String) {
            self.0.lock().unwrap().push(title);
        }
    }
    let mut doc = load_with(Some(Permissions::default()));
    let titles = Arc::new(Titles::default());
    doc.base_mut().set_shell_provider(titles.clone());
    doc.js()
        .eval_string("document.title = 'Set by the app'")
        .unwrap();
    doc.settle();
    click(&mut doc, "#about");
    let seen = titles.0.lock().unwrap().clone();
    assert_eq!(
        seen.first().map(String::as_str),
        Some("Set by the app"),
        "{seen:?}"
    );
    assert_eq!(seen.last(), Some(&doc.title().unwrap()), "{seen:?}");
}

/// Once SvelteKit has shown the first page, nothing happens until something
/// does: no ops, no frames, no version polling (`withSepal()` turns it off).
#[test]
fn idle_turns_do_no_work() {
    use blitz_dom::Document;

    let mut doc = load();
    doc.wait(1.0);
    for _ in 0..3 {
        assert!(!doc.poll(None), "poll reports no change while idle");
        assert_eq!(doc.last_turn().ops, 0);
        assert!(!doc.last_turn().frame, "and asks for no frame");
    }
    assert_eq!(
        doc.js().eval_string("String(__sb.next_timer())").unwrap(),
        "-1",
        "no timer is waiting"
    );
}
