# Patched copy of blitz-dom 0.3.0-beta.4

This is the crates.io source of `blitz-dom` 0.3.0-beta.4 (MIT OR Apache-2.0,
https://github.com/DioxusLabs/blitz) with fifteen changes. `crates/host/Cargo.toml`
swaps it in through `[patch.crates-io]`.

## The changes

**1. Inline SVG colours.** `src/node/serialize.rs`, in `write_outer_html_in_style`: the colour substituted
for `currentColor` when an inline `<svg>` is handed to usvg is converted to
legacy sRGB first (`into_srgb_legacy()`), so it serialises as `rgb(...)`.

Without it, a colour computed in another colour space serialises as
`lab(...)` or `oklch(...)`. usvg cannot parse those: strokes disappear and
fills turn black. Tailwind v4's whole palette is affected.

**2. No panic on an unresolvable URL.** `src/document.rs`, `resolve_url`:
upstream panics when a URL cannot be resolved against the document's base URL.
That happens for a malformed URL (`<img src="http://[">`), and for any relative
URL when the base is the default `data:` URL. The URL comes from app content,
so it now resolves to `about:invalid` instead, which the net provider rejects.

**3. Hit testing a box in a line of text.** `src/node/node.rs`, `hit_inner`:
when a node lays its children out as a line (an "inline root"), the point is
moved into the node's content box, because the text layout is measured from
there. Upstream moves it back for out-of-flow children only. Boxes in the
line (a `<button>` between words, any `inline-block`) are placed relative to
the border box too, so they were tested as if shifted right and down by the
container's padding and border: a button in a `padding: 24px` box only took
clicks 24 px below and to the right of where it was drawn. The point is now
moved back for every child. `tests/shadcn.rs` has the test.

**4. A press moves the focus.** `src/events/pointer.rs`, `handle_pointerdown`
and `handle_click`: in a browser the focus moves when the mouse button goes
down, to the nearest ancestor that can take it (a button, a link, anything
with `tabindex`), or away from whatever had it. Upstream moves it on `click`,
only to text inputs, checkboxes and a few others, and clears it after any
click that matched none of those. So a clicked `<button>` never had the focus
(no keys reached it), and a click that landed somewhere else, or was
synthesised from the keyboard, took the focus away from the element that had
just been given it. The press now does it, without a focus ring, and
`handle_click` no longer clears. Component libraries depend on this: Bits UI
handles a Select's arrow keys on its trigger, which has the focus because it
was pressed.

**5. `:focus-visible` matches.** `src/stylo.rs`: upstream answers `false` for
this pseudo-class, so no focus ring styled with it ever shows, which is every
Tailwind `focus-visible:` utility. It now matches elements whose state has
`FOCUSRING`, which Blitz already sets on keyboard focus (and change 4 leaves
off for a press). `:focus-within` is still always false.

**6. Asking what is animating.** `src/stylo.rs`, `BaseDocument::animations_end`:
a new public method that says when the last CSS animation or transition
running on a node ends. The animation sets are private to the crate, and the
host needs this to answer `element.getAnimations()`, which component libraries
wait on before they remove an element that is animating out.

**7. Siblings are restyled only when they can depend on each other.**
`src/mutator.rs`, `set_attribute` and `clear_attribute`: upstream restyles the
whole subtree of the parent whenever an attribute is set, in case a selector
such as `a + b` or `a ~ b` styles a sibling by it. On a list of 1000 rows that
restyles all of them for one class change. Stylo records on the parent whether
a selector with a sibling combinator was ever tried on its children
(`HAS_SLOW_SELECTOR_LATER_SIBLINGS`), so the parent is now only restyled when
that is set. `crates/host/tests/restyle.rs` checks that descendants and
siblings still follow an attribute change, in a family that has such a
selector and in one that does not.

**8. `:has()` works.** Three places.

- `src/document.rs`, next to the other Stylo preferences:
  `layout.css.has-selector.enabled` is set. Without it Stylo drops every rule
  that contains `:has()` while it parses the stylesheet, so upstream styles
  nothing with it. Tailwind's `has-*` and `group-has-*` utilities are such
  rules.
- `src/stylo.rs`, `BaseDocument::restyle_has_anchors`: when an element's
  attributes, state or children change, the elements whose `:has()` may now
  match differently are restyled. Those are its ancestors and, for
  `:has(+ x)` and `:has(~ x)`, the siblings before each of them. Stylo flags
  every element it has tried a `:has()` selector on, and whether the element
  is the subject (`a:has(b)`, restyle it alone) or not (`a:has(b) c`, restyle
  below and after it); two process-wide flags record whether any such
  selector exists, so a page without `:has()` pays nothing.
- `src/mutator.rs` and `src/document.rs` (`snapshot_node_and`): the calls to
  it, from attribute changes, from children being added or removed, and from
  state changes such as focus and `:checked`.

`crates/host/tests/restyle.rs` has the test.

**9. Adding and removing children restyles what depends on it, no more and
no less.** `src/mutator.rs`, the function behind `append_children` and
`insert_nodes_before`, and `remove_node`:

- Upstream restyles everything under the parent when a child is added. A
  dialog appended to `<body>` restyles the whole page. Now only the added
  nodes are styled, unless a selector that depends on a child's position
  (`:first-child`, `:last-child`, `:nth-child()`, `a + b`) or on there being
  children (`:empty`) was tried on that parent's children, which Stylo
  records on the parent. Then the parent's subtree is restyled as before.
- Upstream restyles nothing when a child is removed, so `:last-child`,
  `:nth-child()` and `:empty` kept matching what they matched before the
  removal. With the same condition, the parent's subtree is now restyled.

Opening a Select from the shadcn example (its list is appended to `<body>`
and measured seven times): best of 8 runs, 39.7 ms before, 20.1 ms after.
`crates/host/tests/restyle.rs` has the test.

Upstream `main` has the same code for (1) as of 22 Nov 2026; (2) to (5) and (7) to (9) were not checked; (6) is an addition, not a fix. Remove this directory and
the `[patch]` entry once a release contains the fix.

**10. A bare `checked` ticks the box.** `src/mutator.rs`,
`set_input_checked_state`: upstream parses the value as a bool, so `checked=""`
(what Svelte sends for `checked={true}`, and what HTML means by a bare
`checked`) failed to parse and was ignored on an input that already existed:
the box stayed unticked. The empty string now means checked. `"true"` and
`"false"` work as before.

Changes 11 to 13 make a small change to a large page cost what it changes.
Measured on the tracker example with 5,000 rows shown (195,000 nodes),
QuickJS, release: `j` (a class and a `data-` attribute on two rows) went
from 64 to 33 ms, the rest being the app's own JavaScript; the page's layouts
in that turn from 21 ms to under 1. Opening a Select went from 521 to 268 ms;
what is left is restyling every element once for the `pointer-events: none`
Bits UI puts on `<body>`, which is inherited, so every element's style does
change. On a page of 200 rows a Select went from 32 to 23 ms.
`profile_every_row_shown` in `crates/host/tests/tracker/bench.rs` measures it.

**11. An attribute that only styles restyles only its element.**
`src/mutator.rs`, `set_attribute`, `clear_attribute` and
`acts_through_styles_only`: upstream restyles the element's whole subtree and
marks it all for rebuilding and relayout whenever any attribute is set or
removed. For `class`, `style`, `data-*` and `aria-*` on an HTML element the
element is now only restyled, and Stylo takes it from there: the snapshot
taken before the change restyles the descendants and siblings whose
selectors depend on it, children are restyled when an inherited value
changes, and the damage comes from comparing the old and new styles, as it
already does for `:hover`. So a background change repaints instead of
relaying out the page, and `overflow: hidden` on `<body>` no longer restyles
every element. Other attributes, and anything inside an `<svg>` (whose
attributes are the source of the drawing), work as upstream.
`crates/host/tests/restyle.rs` has the test.

**12. An unchanged inline SVG is not parsed again.** `src/layout/construct.rs`,
in `collect_layout_children`, and `src/node/svg.rs` (`inline_source`): an
inline `<svg>` is drawn from a source serialised from its subtree, and
upstream parses it again whenever anything in the subtree was restyled. The
source is now kept, and a drawing whose source did not change is kept too.
`pointer-events` on `<body>` used to re-parse all 10,000 icons of the
tracker: 60 ms.

**13. A resolve that changed nothing lays nothing out.** `src/resolve.rs`
and `src/document.rs` (`laid_out_in`):

- `resolve_layout`: upstream runs Taffy from the root and rounds every
  node's box on every resolve, 4 ms on a page of 195,000 nodes when nothing
  changed (Taffy's caches make the layout itself cheap; the rounding is
  not). Any change to a box clears the layout caches from it up to the root,
  so when the root still has its cache and the viewport is the same, the
  pass is skipped.
- `resolve_layout_children`: upstream visits every node to rebuild the box
  tree; subtrees without damage are now skipped, by the same test
  `propagate_damage_flags` uses (1.5 ms on that page).

**14. A box that clips its overflow clips clicks too.** `src/node/node.rs`,
`hit_inner`, with `clips_descendants`: an element that clips its overflow
(as blitz-paint decides in `render.rs`, `should_clip`: `overflow` other than
`visible` on either axis, or paint containment; never the root element)
draws nothing of its descendants outside its padding box, and now nothing
there is hit either. Outside the padding box the element itself is hit on
its border, and nothing otherwise.

Upstream tests the descendants anyway, and in the box's scrolled
coordinates, where a point above a list scrolled down by 40 px counts as
inside it. Rows scrolled out of the top of a list took the clicks meant for
the buttons drawn above it (`examples/counter`: scroll the list a little and
"start timer" stops working), and text overflowing an `overflow: hidden` box
took clicks outside the box (SvelteKit's route announcer, which
`sepal/adapter` gives `pointer-events: none` for that reason). Tests that
clicked rows far below the window scroll them into view first now
(`SvelteDocument::scroll_into_view`).

**15. The layout tree's debug check looks only at the layout tree.**
`src/resolve.rs`, `assert_layout_parents_consistent` (debug builds only):
upstream checks the layout children of every node in the document. Below a
`display: none` box, `resolve_layout_children` never walks, because the box
has no layout children. The lists down there keep what they held when the
box was hidden, and a child freed since is no longer in the slab. Nothing
lays out, paints or hit-tests those lists, and showing the box again
rebuilds them. The check now walks the tree that is kept, from the root.

Upstream panicked on it ("layout child … is not in the slab") in every
debug build: the Tracker's list column hides behind the detail pane in a
narrow window, the rows' labels change, and V8 collects the old ones
(`ops::tests::a_node_freed_below_a_hidden_box_is_not_a_layout_child`).

## Updating

Do not edit anything else here. To move to a new blitz-dom version, replace the
directory with that version's source and reapply the fifteen changes (search for
`sepal patch`).
