# Patched copy of anyrender_vello_hybrid 0.11.0

This is the crates.io source of `anyrender_vello_hybrid` 0.11.0 (MIT OR
Apache-2.0, https://github.com/DioxusLabs/anyrender), the renderer Blitz draws
windows with, with eleven changes. `crates/host/Cargo.toml` swaps it in through
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
offset (its PATCH.md, change 2), over the last frame, kept in a texture of
its own (change 9; `TargetInit::SrcOver`). A frame that changes nothing on
screen draws nothing and returns at once; `skipped_last_frame()` says so.
Presenting is what paced the redraw loop (it waits for a drawable), and
something animating without changing pixels asks for a redraw after every
frame, so the caller has to put the next redraw off itself: the host does, to
the next 60 Hz frame, waiting in the OS (`defer_redraw` in its `window.rs`).
Waiting here instead held up input behind every skipped frame. Full repaints:
first frame, resize, new surface, `repaint_all()` (the host calls it when the
window comes on screen: macOS drops a frame presented before that), over 500
changed commands, or a region over half the window. Commands drawn
from an app-registered texture count as changed every frame. Commands with an
empty box are left out (blitz-paint emits border paths whose points are all
NaN; they draw nothing and never compare equal). A layer with a filter is
one unit: blitz-paint widens its clip by how far the filter
reaches, and every command inside takes that clip as its box, since a change to
any of them changes the filter's output past their own boxes (a drop shadow
whose colour changed left its old shadow behind). A region that touches such a
layer grows to all of it, since drawing part of a filter's input gives a
different output at the cut (a hover next to a shadow erased the part of the
shadow inside the region). A backdrop filter counts only while this renderer
draws backdrop filters, which it does not (`filters::BACKDROP_FILTERS_DRAWN`):
until then such a layer draws as if it had no filter, and counting it made
every change under the Tracker's dialog overlay (`backdrop-blur-xs` over the
whole window) a full repaint.

Partial frames rely on an opaque base colour: the region is drawn
`SrcOver` the last frame, starting with the base colour's fill, which covers
what was there. A window with a transparent base colour would blend the new
region over the old one; it would have to clear the region first. Partial frames draw without
the depth buffer (clearing and storing a window-sized one costs more than the
overdraw it saves there). `SVELTE_BLITZ_DAMAGE=0` turns it off;
`SVELTE_BLITZ_TRACE=1` prints each frame's damage.

Measured on the Tracker (release, 1200x800 at 2x, GPU time from the driver's
per-client `accumulatedGPUTime`): a clock repainting once a second 29 -> 18 ms
over 20 s, the New issue dialog 103 -> 68 ms, switching four views 97 -> 69 ms,
Show all and scrolling 583 -> 269 ms GPU and 2.5 -> 1.9 s CPU. Cost: the
kept frame, ~15-20 MB. Window captures of seven states (list, hover,
dialog open and closed, Board, Insights, list again) match a full repaint
exactly or within 1/255 on a few dozen antialiased edge pixels of what changed
(vello reaches them through a different path in a small scene); a partial frame
starts from the opaque background fill, so that does not accumulate.

On this GPU the cost of a frame is mostly per command buffer (~0.15 ms each at
the clocks an idle app runs at), not per pixel or per pass; wgpu submits about
five a frame (pending writes, two for the passes, the copy to the surface,
transitions). That, not drawing, is most of what a partial frame still costs.

**7. A switch for the depth buffer.** `src/window_renderer.rs`,
`depth_texture()`: `SVELTE_BLITZ_DEPTH=0` draws without vello_gpu's depth
buffer (Depth32Float on Apple GPUs, 15 MB at 2400x1600), in paint order. On by
default. It cannot be memoryless (which would cost no memory): vello_gpu draws
all opaque strips first, in a pass of their own, and later passes load that
depth to keep alpha strips hidden behind them, so it has to outlive a pass.
Partial frames (change 6) already draw without it. Measured on the Tracker
(release, 1,000 issues, 2 runs each, the compare harness): without it, 15-17
MB less GPU memory and ~14 MB less RAM in every scenario; GPU time the same
within noise for the clock, the New issue dialog and view switches (mostly
partial frames), but Show all + scrolling 533 -> 768 ms (+44%, ~0.6 ms more
per full repaint). Kept on until scrolling stops being full repaints.

**8. A check mode for damage tracking.** `src/window_renderer.rs`,
`verify_frame` and `draw_region`, and `damage::verify()`:
`SVELTE_BLITZ_DAMAGE=verify` draws every frame that was not drawn in full a
second time, in full, into a texture of its own (the window's renderer, so the
same glyph atlas and settings), reads both back and prints a `[verify]` line
per frame: `same as a full repaint (largest difference N/255)`, or `DIFFERS`
with how many pixels differ by more than 2/255, the rectangle they are in, and
every command drawn there. The kept frame gets `COPY_SRC` for it. Reading back
is a blit, so this mode costs the driver memory change 1 of wgpu-hal avoids,
and a frame takes tens of milliseconds more: for checking, never for measuring.

**9. The kept frame is ours, copied to the surface in the frame's submit.**
`src/window_renderer.rs`, `SurfaceCopy` and `Kept`: with damage tracking
(change 6) the last frame used to be kept in wgpu_context's intermediate
texture, which `maybe_blit_and_present` copies to the surface with wgpu's
`TextureBlitter`, in a submit of its own and in a pass that loads the surface
texture; a surface texture is new every frame, so wgpu cleared it first, in a
full-window pass of its own. The renderer now owns the kept frame (created on
resume and resize) and copies it to the surface texture at the end of the
frame's own command encoder, in a pass that clears instead of loading. The
surface is created without an intermediate texture on Apple (elsewhere, a
`PostMultiplied` surface still has one, for its alpha conversion).

A clock tick is one submit of 3 command buffers plus the present, 5 render
passes instead of 6, and no full-window clear. Measured against the code
before it in one interleaved session (compare harness, 1,000 issues, 3 runs
each): switching views 61.9 -> 39.4 ms GPU and 382 -> 340 ms CPU; the clock,
the New issue dialog and scrolling the same within noise. Window captures of
seven states (list, hover, New issue open and closed, Board, Insights, list
again) match `SVELTE_BLITZ_DAMAGE=0`.

**10. The kept frame and the depth buffer are let go when idle.**
`src/window_renderer.rs`, `release_if_idle()` and the start of `render`: after
two seconds without a frame presented, the kept frame (change 9) and the depth
buffer (2400x1600 each, 15 MB each) are dropped; the next frame makes them again
and is drawn in full, as nothing keeps the last frame any more. The renderer
never wakes for it: the host asks in `about_to_wait`, which winit runs whenever
anything woke the loop (an event, a timer of the app's, a host call), so a
window that nothing wakes keeps them.

Measured against the build before it, interleaved (compare harness, 1,000
issues, 3 runs each): idle, RAM 119 -> 88 MB and GPU memory 59 -> 28 MB at the
end, RAM 5 s after load 124 -> 94 MB in every scenario; the first wake-up in
the bench Tracker is its own toast timer, 5 s after start. A window shown again
after it repaints correctly (captures match `SVELTE_BLITZ_DAMAGE=0`).

**11. A layer that only groups is drawn as a clip.** `src/scene.rs`,
`push_layer`: a layer with alpha 1, normal blending and no filter this renderer
draws (backdrop filters are not drawn at all) composites exactly like its
contents drawn directly, so it is pushed as a clip path, without a layer.
vello_gpu sizes its layer texture for the largest layer of a frame: the
Tracker's dialog overlay, a layer only because it has `backdrop-filter`, made
it 2400x1600 (15 MB) for as long as the dialog was open. Window captures with
the dialog open are identical before and after (0 pixels differ).

This and change 6's rule for backdrop filters rely on one check,
`filters::BACKDROP_FILTERS_DRAWN`; both go away when backdrop filters are drawn.

Measured against the code before it, interleaved (compare harness, 1,000
issues, 3 runs each): New issue dialog 123 -> 78 ms GPU, 350 -> 326 ms CPU;
scrolling 622 -> 574 ms GPU; the clock and view switches the same within
noise. With the dialog open, wgpu's textures and buffers 53.6 -> 39.6 MB
(`compare/results/dialog-memory.md`).

## Updating

Do not edit anything else here. To move to a new version, replace the
directory with that version's source and reapply the six changes (search for
`svelte-blitz patch`; `offscreen.rs` is new as a whole).
