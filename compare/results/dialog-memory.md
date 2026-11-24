# GPU memory with the New issue dialog open

The bench Tracker, 1,000 issues, window 1200x800 at 2x, the clock ticking (so frames are being
drawn, as in the scenarios), snapshot 3 s after opening New issue with `c`. wgpu's textures and
buffers come from `SVELTE_BLITZ_GPU_LOG=1` (wgpu-hal change 2) replayed by
`compare/harness/gpu_live.py`; the rest is `footprint` at the same moment. Single snapshots: the
driver's categories move by ~10-15 MB between runs.

## Everything wgpu holds, largest first

| | clock only | clock + dialog (before) | clock + dialog (after the fix) | what it is | needed |
|---|---|---|---|---|---|
| kept frame, 2400x1600 BGRA8Unorm | 15.1 MB | 15.1 MB | 15.1 MB | damage tracking's last frame (anyrender_vello_hybrid change 9) | yes; let go when idle (change 10) |
| depth texture, 2400x1600 Depth32Float | 15.1 MB | 15.1 MB | 15.1 MB | vello_gpu's depth test for opaque strips, full frames only | trades 15 MB for scroll speed; kept on by your decision (`SVELTE_BLITZ_DEPTH=0` drops it); let go when idle |
| layer texture, RGBA8Unorm | — | 15.1 MB (2400x1600) | 1.0 MB (512x512) | vello_gpu's intermediate texture for layers (opacity, blending, filters), sized for the largest layer of the frame | the 2400x1600 one was waste: see below |
| atlas page, 1024x1024 RGBA8Unorm | 4.0 MB | 4.0 MB | 4.0 MB | glyphs and images (change 4) | yes |
| alpha texture, 4096xN RGBA32Uint | 1.0 MB (N=16) | 2.0 MB (N=21) | 2.0 MB | this frame's strip coverage masks; grows with what is drawn | yes |
| strips arena (buffer) | 0.8 MB | 1.6 MB | 1.6 MB | this frame's strips | yes |
| wgpu's zero-init buffer | 0.5 MB | 0.5 MB | 0.5 MB | wgpu internal, for zero-initialising resources | yes (wgpu's) |
| encoded paints, filter data, 4096xN RGBA32Uint | 0.1 MB | 0.2 MB | 0.2 MB | paints and filter parameters | yes |
| 6 more under 64 KB | — | — | — | uniforms, placeholders | yes |
| **total** | **36.7 MB** | **53.6 MB** | **39.6 MB** | | |

Outside wgpu, same moments: the window's surface (IOSurface) 30 MB in every state (two
2400x1600 drawables, the CAMetalLayer's); the driver's own memory ("owned physical footprint
(unmapped) (graphics)" plus "IOAccelerator (graphics)"): 176 MB clock only, 209 MB with the
dialog before the fix, 190 MB after. The driver's part is what the 120 MB floor in wgpu-hal's
PATCH.md refers to; it grows with the work per frame and is returned 1-2 s after the last frame.

## Layers drawn with the dialog open

21 `PushLayer`s (anyrender layers with opacity, blending or a filter; plain clips not counted):

- 19 at alpha 0.30, 8x8 to 20x26 device px: icons in the list rows with `opacity` below 1
  (blitz-paint draws an element with opacity < 1 as a layer). Also there with the clock only.
  Needed: a group opacity differs from per-shape alpha where an icon's shapes overlap.
- 1 at alpha 0.30 inside the dialog: an icon, as above.
- 1 at alpha 1.00, normal blending, no filter, the whole window: the dialog overlay
  (`dialog-overlay.svelte`: `bg-black/10 ... supports-backdrop-filter:backdrop-blur-xs`).
  blitz-paint makes it a layer because it has a backdrop filter; vello_hybrid does not draw
  backdrop filters, so the layer only groups, and an opaque, normally blended group
  composites exactly like its contents drawn directly. It cost vello_gpu a 2400x1600 layer
  texture and the passes that fill and composite it. **Fixed** (anyrender_vello_hybrid change
  11): such a layer is drawn as a clip. Window captures with the dialog open are identical to
  before (0 pixels differ), and match `SVELTE_BLITZ_DAMAGE=0`.

Nothing else here is plainly waste. What remains above the clock-only state is the dialog's
own drawing (bigger strip buffers and alpha masks) and the driver's memory for the extra work.

## Measured, interleaved (compare harness, 3 runs each, session `j5-overlay`)

| | HEAD | overlay layer drawn as a clip |
|---|---|---|
| New issue dialog: GPU time | 123.3 ms (112.4-130.1) | 78.1 ms (72.0-102.2) |
| New issue dialog: CPU time | 350 ms (261-352) | 326 ms (307-331) |
| New issue dialog: GPU memory at the end | 289 MB | 289 MB |
| Show all + scroll: GPU time | 622 ms (613-635) | 574 ms (574-578) |
| clock, view switches | the same within noise | |

GPU memory at the end of the dialog scenario does not move: by then the dialog is closed, and
vello_gpu's layer textures shrink once the large layer is gone (vello_gpu change 1). What
remains of the gap to the clock's 203 MB at that moment is the driver's own memory, grown by
the dialog's heavier frames (most are full repaints) and returned 1-2 s after the last frame.
The snapshot above, taken with the dialog open, shows what the change saves while it is open:
wgpu's textures and buffers 53.6 -> 39.6 MB, the driver's 209 -> 190 MB.
