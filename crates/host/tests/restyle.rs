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

const CHILD_CSS: &str = "
    div { height: 10px; width: 10px }
    .plain > .item { width: 20px }
    .plain .deep { width: 25px }
    .edges > .item:first-child { width: 30px }
    .edges > .item:last-child { width: 40px }
    .nth > .item:nth-child(2) { width: 50px }
    .pairs > .item + .item { width: 60px }
    .hollow:empty { width: 70px }
    .from .moved { width: 80px }
    .to .moved { width: 90px }
";

/// Adding and removing children: what was added is styled, and so is
/// everything whose style depends on where it sits among its siblings.
/// Upstream restyles all of the parent's subtree for that; the patched copy
/// only when such a selector is in play.
#[test]
fn children_that_come_and_go_are_styled_along_with_what_depends_on_them() {
    let js = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/shadcn/dist/app.js"
    ))
    .expect("run `pnpm run build` in the repository root first");
    let mut doc = SvelteDocument::new(&js, CHILD_CSS, Default::default()).unwrap();
    doc.layout(800, 600);
    let eval = |doc: &mut SvelteDocument, code: &str| {
        doc.js().eval_string(code).unwrap();
        doc.settle();
        doc.resolve();
    };
    let width = |doc: &SvelteDocument, id: &str| {
        doc.rect(
            doc.query(&format!("#{id}"))
                .unwrap_or_else(|| panic!("no #{id}")),
        )
        .unwrap()
        .2
    };
    eval(
        &mut doc,
        "(() => {
            globalThis.add = (parent, id, name = 'item', before = null) => {
                const node = document.createElement('div');
                node.className = name;
                node.id = id;
                document.getElementById(parent).insertBefore(node, before && document.getElementById(before));
                return node;
            };
            document.body.textContent = '';
            for (const name of ['plain', 'edges', 'nth', 'pairs', 'hollow', 'from', 'to']) {
                const node = document.createElement('div');
                node.className = name;
                node.id = name;
                document.body.appendChild(node);
            }
            return '';
        })()",
    );
    assert_eq!(width(&doc, "hollow"), 70.0);

    // Nothing here depends on position: the new nodes are styled, with what is in them.
    eval(
        &mut doc,
        "add('plain', 'p1'); add('p1', 'p1deep', 'deep'); ''",
    );
    assert_eq!((width(&doc, "p1"), width(&doc, "p1deep")), (20.0, 25.0));
    eval(&mut doc, "add('plain', 'p2'); ''");
    assert_eq!((width(&doc, "p1"), width(&doc, "p2")), (20.0, 20.0));

    // `:first-child` and `:last-child` move to whoever is first and last now.
    eval(&mut doc, "add('edges', 'e1'); ''");
    assert_eq!(width(&doc, "e1"), 40.0, "the only child is the last one");
    eval(&mut doc, "add('edges', 'e2'); ''");
    assert_eq!((width(&doc, "e1"), width(&doc, "e2")), (30.0, 40.0));
    eval(&mut doc, "add('edges', 'e0', 'item', 'e1'); ''");
    assert_eq!(
        (width(&doc, "e0"), width(&doc, "e1"), width(&doc, "e2")),
        (30.0, 10.0, 40.0)
    );
    eval(&mut doc, "document.getElementById('e2').remove(); ''");
    assert_eq!((width(&doc, "e0"), width(&doc, "e1")), (30.0, 40.0));

    // `:nth-child()`: inserting in front renumbers the rest.
    eval(&mut doc, "add('nth', 'n1'); add('nth', 'n2'); ''");
    assert_eq!((width(&doc, "n1"), width(&doc, "n2")), (10.0, 50.0));
    eval(&mut doc, "add('nth', 'n0', 'item', 'n1'); ''");
    assert_eq!(
        (width(&doc, "n0"), width(&doc, "n1"), width(&doc, "n2")),
        (10.0, 50.0, 10.0)
    );

    // `a + b`: a new first child gives the old first one something before it.
    eval(&mut doc, "add('pairs', 'q1'); ''");
    assert_eq!(width(&doc, "q1"), 10.0);
    eval(&mut doc, "add('pairs', 'q0', 'item', 'q1'); ''");
    assert_eq!((width(&doc, "q0"), width(&doc, "q1")), (10.0, 60.0));

    // `:empty` stops matching, and matches again.
    eval(&mut doc, "add('hollow', 'h1'); ''");
    assert_eq!(width(&doc, "hollow"), 10.0);
    eval(&mut doc, "document.getElementById('h1').remove(); ''");
    assert_eq!(width(&doc, "hollow"), 70.0);

    // A node that moves is styled by where it is now.
    eval(&mut doc, "add('from', 'm', 'moved'); ''");
    assert_eq!(width(&doc, "m"), 80.0);
    eval(
        &mut doc,
        "document.getElementById('to').appendChild(document.getElementById('m')); ''",
    );
    assert_eq!(width(&doc, "m"), 90.0);
}
