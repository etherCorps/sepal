# Patched copy of anyrender_vello_hybrid 0.11.0

This is the crates.io source of `anyrender_vello_hybrid` 0.11.0 (MIT OR
Apache-2.0, https://github.com/DioxusLabs/anyrender), the renderer Blitz draws
windows with, with six changes. `crates/host/Cargo.toml` swaps it in through
`[patch.crates-io]`. All are marked `svelte-blitz patch`.

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

**4. Small atlas pages.** `src/window_renderer.rs`, `render_settings()`:
the atlas that holds cached glyphs (change 1) and images defaults to pages of
4096 x 4096, 64 MB each, the first allocated with the first cached glyph. A
page of text needs a few hundred glyphs, so it sat mostly empty. Pages are now
1024 x 1024 (4 MB), added as they fill; 128 of them hold as much as the
default's 8. Measured on the Tracker (1200x800 window on a 2x display, a
clock repainting once a second, `footprint`): 448 MB with 4096 pages, 383 MB
with 2048, 371 MB with 1024, the same as with no glyph atlas at all.

**5. Images larger than an atlas page get a texture of their own.**
`src/scene.rs`, `ImageManager::image_source` and `OwnTexture`; their textures
are bound with the app's in `window_renderer.rs` and `offscreen.rs`. vello_gpu's
`upload_image` unwraps the atlas allocation, so an image larger than a page
panicked the renderer (`TextureTooLarge`): with change 4 any image over 1024 px,
before it any over 4096 px. Such an image is now written to its own texture
and drawn as an external texture at its natural size. One larger than the
device's texture limit is not drawn. Images up to a page still go to the
atlas. Checked with a 4500 x 300 PNG as a CSS background: `gpushot:` matches
`shot:` except on glyph edges, and 1024 and 1025 px images both draw.

**6. Repaint only what changed.** `src/damage.rs` (new), the recording mode of
`VelloHybridScenePainter` (`recorder`, `src/scene.rs`) and `render` in
`src/window_renderer.rs`. Each frame's drawing commands are recorded (anyrender's
`recording::Scene`), compared in order with the last frame's (Myers' diff), and
only the commands that touch the region whose commands changed are drawn, moved
so the region's corner is the scene's origin and placed with vello_gpu's root
offset (its PATCH.md, change 2), over the last frame kept in the surface's
intermediate texture (`TargetInit::SrcOver`). A frame that changes nothing on
screen draws nothing and returns at once; `skipped_last_frame()` says so.
Presenting is what paced the redraw loop (it waits for a drawable), and
something animating without changing pixels asks for a redraw after every
frame, so the caller has to put the next redraw off itself: the host does, to
the next 60 Hz frame, waiting in the OS (`defer_redraw` in its `window.rs`).
Waiting here instead held up input behind every skipped frame. Full repaints: first frame, resize, new surface,
over 500 changed commands, or a region over half the window. Commands drawn
from an app-registered texture count as changed every frame. Commands with an
empty box are left out (blitz-paint emits border paths whose points are all
NaN; they draw nothing and never compare equal). Partial frames draw without
the depth buffer (clearing and storing a window-sized one costs more than the
overdraw it saves there). `SVELTE_BLITZ_DAMAGE=0` turns it off;
`SVELTE_BLITZ_TRACE=1` prints each frame's damage.

Measured on the Tracker (release, 1200x800 at 2x, GPU time from the driver's
per-client `accumulatedGPUTime`): a clock repainting once a second 29 -> 18 ms
over 20 s, the New issue dialog 103 -> 68 ms, switching four views 97 -> 69 ms,
Show all and scrolling 583 -> 269 ms GPU and 2.5 -> 1.9 s CPU. Cost: the
intermediate texture, ~15-20 MB. Window captures of seven states (list, hover,
dialog open and closed, Board, Insights, list again) match a full repaint
exactly or within 1/255 on a few dozen antialiased edge pixels of what changed
(vello reaches them through a different path in a small scene); a partial frame
starts from the opaque background fill, so that does not accumulate.

On this GPU the cost of a frame is mostly per command buffer (~0.15 ms each at
the clocks an idle app runs at), not per pixel or per pass; wgpu submits about
five a frame (pending writes, two for the passes, the copy to the surface,
transitions). That, not drawing, is most of what a partial frame still costs.

## Updating

Do not edit anything else here. To move to a new version, replace the
directory with that version's source and reapply the six changes (search for
`svelte-blitz patch`; `offscreen.rs` is new as a whole).
