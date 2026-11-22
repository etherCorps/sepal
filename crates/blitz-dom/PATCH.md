# Patched copy of blitz-dom 0.3.0-beta.4

This is the crates.io source of `blitz-dom` 0.3.0-beta.4 (MIT OR Apache-2.0,
https://github.com/DioxusLabs/blitz) with three changes. `crates/host/Cargo.toml`
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

Upstream `main` has the same code for (1) as of 22 Nov 2026; (2) and (3) were not checked. Remove this directory and
the `[patch]` entry once a release contains the fix.

## Updating

Do not edit anything else here. To move to a new blitz-dom version, replace the
directory with that version's source and reapply the three changes (search for
`svelte-blitz patch`).
