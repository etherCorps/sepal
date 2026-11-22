//! When an attribute changes, every element whose style depends on it has to
//! be restyled: the element, its descendants, and the siblings after it.
//! Blitz upstream restyles the whole subtree of the parent on every attribute
//! that is set, to be safe; the patched copy only does when a selector that
//! reaches across siblings was used there (see crates/blitz-dom/PATCH.md).
//! This is the test that it still restyles enough.
//!
//! And `:has()`, which upstream does not parse at all: an element styled by
//! what is inside or after it has to follow when that changes.
//!
//! Uses the shadcn example's bundle for its DOM layer, with a stylesheet of
//! its own. Build the bundle first: `pnpm run build` in the repository root.

use svelte_blitz_host::SvelteDocument;

const CSS: &str = "
    div { height: 10px; width: 10px }
    .a[data-on] { width: 20px }
    .a[data-on] > .kid { width: 30px }
    .a[data-on] .deep { width: 35px }
    .a[data-on] + .next { width: 40px }
    .a[data-on] ~ .later { width: 50px }
    .other[data-on] { width: 15px }
    .a.on2 + .next { height: 44px }
";

const SETUP: &str = "
    (() => {
        const el = (name, parent) => {
            const node = document.createElement('div');
            node.className = name;
            node.id = name;
            parent.appendChild(node);
            return node;
        };
        document.body.textContent = '';
        const outer = el('outer', document.body);
        const wrap = el('wrap', outer);
        el('before', wrap);
        const a = el('a', wrap);
        el('deep', el('kid', a));
        el('next', wrap);
        el('later', wrap);
        // A second family, with no selector that reaches across siblings.
        const plain = el('plain', outer);
        el('other', plain);
        el('unrelated', plain);
        return '';
    })()
";

fn widths(doc: &SvelteDocument) -> Vec<(String, f64)> {
    [
        "a",
        "kid",
        "deep",
        "before",
        "next",
        "later",
        "other",
        "unrelated",
    ]
    .iter()
    .map(|id| {
        let node = doc.query(&format!("#{id}")).unwrap();
        (id.to_string(), doc.rect(node).unwrap().2)
    })
    .collect()
}

fn named(pairs: &[(&str, f64)]) -> Vec<(String, f64)> {
    pairs.iter().map(|(id, w)| (id.to_string(), *w)).collect()
}

#[test]
fn an_attribute_change_restyles_everything_that_depends_on_it() {
    let js = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/shadcn/dist/app.js"
    ))
    .expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::new(&js, CSS, Default::default()).unwrap();
    doc.layout(800, 600);
    let eval = |doc: &mut SvelteDocument, code: &str| {
        doc.js().eval_string(code).unwrap();
        doc.settle();
        doc.resolve();
    };
    eval(&mut doc, SETUP);

    let off = named(&[
        ("a", 10.0),
        ("kid", 10.0),
        ("deep", 10.0),
        ("before", 10.0),
        ("next", 10.0),
        ("later", 10.0),
        ("other", 10.0),
        ("unrelated", 10.0),
    ]);
    let on = named(&[
        ("a", 20.0),
        ("kid", 30.0),
        ("deep", 35.0),
        ("before", 10.0),
        ("next", 40.0),
        ("later", 50.0),
        ("other", 15.0),
        ("unrelated", 10.0),
    ]);
    assert_eq!(widths(&doc), off);

    // Set, remove, and set again: a first style pass and a later one take
    // different paths.
    for _ in 0..2 {
        eval(
            &mut doc,
            "for (const id of ['a', 'other']) document.getElementById(id).setAttribute('data-on', ''); ''",
        );
        assert_eq!(widths(&doc), on, "after the attribute was set");
        eval(
            &mut doc,
            "for (const id of ['a', 'other']) document.getElementById(id).removeAttribute('data-on'); ''",
        );
        assert_eq!(widths(&doc), off, "after it was removed");
    }

    // A class is an attribute too.
    let next_height = |doc: &SvelteDocument| doc.rect(doc.query("#next").unwrap()).unwrap().3;
    eval(
        &mut doc,
        "document.getElementById('a').classList.add('on2'); ''",
    );
    assert_eq!(next_height(&doc), 44.0);
    eval(
        &mut doc,
        "document.getElementById('a').classList.remove('on2'); ''",
    );
    assert_eq!(next_height(&doc), 10.0);
}

const HAS_CSS: &str = "
    div { height: 10px; width: 10px }
    .card:has(> .flag[data-on]) { width: 100px }
    .page:has(.flag[data-on]) { width: 200px }
    .page:has(.flag[data-on]) .aside { width: 30px }
    .page:has(.flag[data-on]) ~ .footer { width: 40px }
    .label:has(+ .flag[data-on]) { width: 50px }
    .card:has(.extra) { height: 60px }
    .card:has(:focus) { height: 70px }
";

const HAS_SETUP: &str = "
    (() => {
        const el = (name, parent) => {
            const node = document.createElement('div');
            node.className = name;
            node.id = name;
            parent.appendChild(node);
            return node;
        };
        document.body.textContent = '';
        const page = el('page', document.body);
        const card = el('card', page);
        el('label', card);
        el('flag', card).setAttribute('tabindex', '0');
        el('aside', page);
        el('footer', document.body);
        return '';
    })()
";

#[test]
fn has_follows_what_it_looks_for() {
    let js = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/shadcn/dist/app.js"
    ))
    .expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::new(&js, HAS_CSS, Default::default()).unwrap();
    doc.layout(800, 600);
    let eval = |doc: &mut SvelteDocument, code: &str| {
        doc.js().eval_string(code).unwrap();
        doc.settle();
        doc.resolve();
    };
    let size = |doc: &SvelteDocument, id: &str| {
        let rect = doc.rect(doc.query(&format!("#{id}")).unwrap()).unwrap();
        (rect.2, rect.3)
    };
    let widths = |doc: &SvelteDocument| {
        ["card", "page", "aside", "footer", "label"].map(|id| size(doc, id).0)
    };
    eval(&mut doc, HAS_SETUP);
    assert_eq!(widths(&doc), [10.0; 5]);

    // An attribute on a descendant: the parent (`>`), an ancestor, something
    // under that ancestor, a sibling after it, and the sibling before the
    // descendant (`:has(+ …)`).
    for _ in 0..2 {
        eval(
            &mut doc,
            "document.getElementById('flag').setAttribute('data-on', ''); ''",
        );
        assert_eq!(widths(&doc), [100.0, 200.0, 30.0, 40.0, 50.0]);
        eval(
            &mut doc,
            "document.getElementById('flag').removeAttribute('data-on'); ''",
        );
        assert_eq!(widths(&doc), [10.0; 5]);
    }

    // A child that arrives and leaves.
    eval(
        &mut doc,
        "(() => { const extra = document.createElement('div'); extra.className = 'extra'; extra.id = 'extra'; document.getElementById('card').appendChild(extra); return '' })()",
    );
    assert_eq!(size(&doc, "card").1, 60.0);
    eval(&mut doc, "document.getElementById('extra').remove(); ''");
    assert_eq!(size(&doc, "card").1, 10.0);

    // A state: the focus moving in and out.
    eval(&mut doc, "document.getElementById('flag').focus(); ''");
    assert_eq!(size(&doc, "card").1, 70.0);
    eval(&mut doc, "document.getElementById('flag').blur(); ''");
    assert_eq!(size(&doc, "card").1, 10.0);

    // And an element that is born matching.
    eval(
        &mut doc,
        "(() => { const card = document.createElement('div'); card.className = 'card'; card.id = 'born'; const flag = document.createElement('div'); flag.className = 'flag'; flag.setAttribute('data-on', ''); card.appendChild(flag); document.body.appendChild(card); return '' })()",
    );
    assert_eq!(size(&doc, "born").0, 100.0);
}
