# Patched copy of anyrender_vello_hybrid 0.11.0

This is the crates.io source of `anyrender_vello_hybrid` 0.11.0 (MIT OR
Apache-2.0, https://github.com/DioxusLabs/anyrender), the renderer Blitz draws
windows with, with three changes. `crates/host/Cargo.toml` swaps it in through
`[patch.crates-io]`. Both are marked `svelte-blitz patch`.

## The changes

**1. The glyph atlas is on.** `src/scene.rs`, `draw_glyphs`, and the switch in
`src/lib.rs` (`glyph_cache`, `set_glyph_cache`): `vello_gpu` can rasterise a
glyph once into an atlas and reuse it (`.atlas_cache(true)` on a glyph run).
Upstream does not ask for that, so every glyph on the page is rasterised again
for every frame. On a page of ordinary text that was half of the work of
painting a frame.

Measured on the shadcn example while its Skeleton pulses, 4 s sampled at
1 ms: time in `paint_scene` 290 ms without the atlas, 143 ms with it.

The text is not pixel-identical: about 5500 pixels of a 900 × 700 page differ
by more than 8/255, none by more than 64, all on glyph edges. Enlarged, the
two cannot be told apart. If text looks wrong on some machine, run the host
with `SVELTE_BLITZ_GLYPH_CACHE=0`, which calls `set_glyph_cache(false)`.

**2. Drawing without a window.** `src/offscreen.rs`, exported as
`render_to_rgba`: draws a scene into a texture with the same renderer and
reads the pixels back. Upstream can only draw into a window surface. It is
how the first change was checked, and how to look at what the window's
renderer produces from a terminal:
`cargo run --features screenshot --example drive -- <app dir> gpushot:out.png`
in `crates/host`. It needs a GPU, so no test depends on it.

With the atlas off, its output matches the CPU renderer used for screenshots
and tests to within 2/255 per channel.

**3. Layers as large as the window.** `src/window_renderer.rs`,
`render_settings()`, used by `VelloHybridRendererOptions::default()` and so by
the window and by `render_to_rgba`: vello_gpu caps the textures it draws
layers into at 4096 px (`LayersConfig::max_texture_size`). A layer the size of
the window, such as a Dialog's dimmed backdrop, is 5132 px wide in a
full-screen window on a 5K display at 2x, so the frame failed and `render`
panicked with `IntermediateTexture(TooLarge { .. })`. The patch asks for
`u16::MAX`, which vello_gpu clamps to the device's own limit (16384 on an M4).
A window larger than the device allows still fails. Reproduce with
`drive -- ../../examples/router-dom-probe/dist 5200x2800 click:#open-dialog
time:0.5 gpushot:out.png` in `crates/host`.

## Updating

Do not edit anything else here. To move to a new version, replace the
directory with that version's source and reapply the three changes (search for
`svelte-blitz patch`; `offscreen.rs` is new as a whole).
