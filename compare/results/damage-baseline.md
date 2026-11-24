# svelte-blitz, damage tracking on

Apple M4 Pro, Version 27.0.1 (Build 26A434), the Tracker (bench build, 1,000 issues), window 1200x800 at 2x. Session `p0-base` (damage tracking on, the default), 0 warm-up + 2 measured runs per scenario. Cells: median (min–max).

- GPU time: the driver's per-process `accumulatedGPUTime` (what Activity Monitor shows) during the scenario.
- CPU time: `proc_pid_rusage` during the scenario.
- GPU memory: footprint's graphics categories and IOSurface at the end of the scenario.
- Frames: drawn frames during the scenario, from the trace. Full / partial / nothing changed is damage tracking's verdict; "nothing changed" frames present nothing. With damage tracking off every frame is drawn in full.

## 3: idle (nothing changes) (20 s)

| | damage on |
|---|---|
| GPU time, ms | 0.0 (0.0–0.0) |
| CPU time, ms | 4 (4–5) |
| GPU memory at the end, MB | 60 (60–60) |
| RAM at the end, MB | 136 (136–136) |
| frames presented | 0 (0–0) |
| … full | 0 (0–0) |
| … partial | 0 (0–0) |
| drawn but nothing changed (not presented) | 0 (0–0) |

## 4: clock ticking once a second (20 s)

| | damage on |
|---|---|
| GPU time, ms | 19.5 (18.6–20.3) |
| CPU time, ms | 93 (92–94) |
| GPU memory at the end, MB | 207 (207–207) |
| RAM at the end, MB | 294 (294–294) |
| frames presented | 20 (20–20) |
| … full | 0 (0–0) |
| … partial | 20 (20–20) |
| drawn but nothing changed (not presented) | 0 (0–0) |

## 8: create an issue (New issue dialog) (1 s)

| | damage on |
|---|---|
| GPU time, ms | 69.8 (63.8–75.7) |
| CPU time, ms | 263 (253–274) |
| GPU memory at the end, MB | 294 (291–296) |
| RAM at the end, MB | 398 (396–399) |
| frames presented | 36 (34–38) |
| … full | 17 (16–18) |
| … partial | 19 (18–20) |
| drawn but nothing changed (not presented) | 1 (1–1) |

## 9: Show all, then wheel-scroll the list (9 s)

| | damage on |
|---|---|
| GPU time, ms | 243.6 (225.5–261.7) |
| CPU time, ms | 1,842 (1,831–1,852) |
| GPU memory at the end, MB | 75 (75–76) |
| RAM at the end, MB | 176 (176–177) |
| frames presented | 190 (184–195) |
| … full | 135 (132–138) |
| … partial | 54 (52–57) |
| drawn but nothing changed (not presented) | 184 (180–188) |

## 10: switch views: Board, Insights, Settings, Issues (8 s)

| | damage on |
|---|---|
| GPU time, ms | 59.3 (50.9–67.7) |
| CPU time, ms | 346 (338–354) |
| GPU memory at the end, MB | 75 (75–75) |
| RAM at the end, MB | 197 (196–197) |
| frames presented | 60 (54–67) |
| … full | 4 (4–4) |
| … partial | 56 (50–63) |
| drawn but nothing changed (not presented) | 4 (4–4) |

## Aborted runs

None.

## Baseline notes

- Build: `compare/builds/Tracker-blitz.app` on the host at commit 31673d9 (damage tracking on, skipped frames still sleep out a 60 Hz frame). Raw data: `compare/results/raw/p0-base/` (this branch only).
- Cut to 2 runs per scenario and no warm-up run to fit the time available (the app had just been launched by an aborted attempt, so caches were warm). Treat single-run spreads as indicative.
- GPU memory at the end is driver memory that macOS reclaims 1-2 s after the last frame: 207 MB in scenario 4 (a frame a second), 75 MB in 9 and 10 (frames had stopped).
- Scenario 9: 184 frames drawn with nothing changed. The list reaches its end about halfway through the wheel (the 2x trackpad bug), and every later wheel event asks for a frame. 135 of the 190 frames presented were full repaints: a scroll moves more than half the window.

## One frame, command by command

From `SVELTE_BLITZ_GPU_LOG=1` (wgpu-hal change 2) on the same Tracker at 1,000 issues, clock on. Logs: `raw/p0-base/gpu-log-damage-on.log` and `gpu-log-damage-off.log`. Taken on the host at HEAD with the log added; between 31673d9 and HEAD nothing changed in how a drawn frame is encoded.

**Partial frame (clock tick, damage on, no new glyphs): 3 submits, 10 command buffers, 6 render passes.**

| submit | command buffers | passes and what they write |
|---|---|---|
| 1 | PendingWrites, Render to Texture Pass, Pre Pass, Transit | upload 4096x1 into the 4096x16 RGBA32Uint alpha texture (a pass); upload 4096x1 into the 4096x1 RGBA32Uint texture (a pass); 14 KB into the 842 KB strip buffer (a pass); the scene: 2400x1600 BGRA8Unorm intermediate, load/store, drawn in a 512x60 region, no depth |
| 2 | TextureBlitter::pass, Pre Pass, Transit, Present (wgpu) | wgpu clears the 2400x1600 surface texture (clear/store) because it counts it as uninitialised, then the blitter copies the whole 2400x1600 intermediate over it (load/store) |
| 3 | PendingWrites (empty) | none |
| present | 1 (hal) | none |

A tick that rasterises new glyphs adds one submit of 4 command buffers: a 1024x1024 RGBA8Unorm glyph atlas pass (load/store) and its uploads.

**Full frame (clock tick, `SVELTE_BLITZ_DAMAGE=0`): 2 submits, 13 command buffers, 10 render passes.**

| submit | command buffers | passes and what they write |
|---|---|---|
| 1 | PendingWrites, 3 × (Render to Texture Pass + Pre Pass), Clear Rects + Pre Pass, Transit, Present (wgpu) | uploads, a pass each: 4096x16 into the alpha texture, 4096x1 RGBA32Uint, 59 KB + 3 KB + 765 KB into the 842 KB strip buffer, 224 B config; scene pass 1: surface 2400x1600 BGRA8Unorm clear/store + depth 2400x1600 Depth32Float clear/store; a layer: 512x512 RGBA8Unorm load/store; scene pass 2: surface load/store + depth load/store; Clear Rects: 512x512 layer texture load/store |
| 2 | PendingWrites (empty) | none |
| present | 1 (hal) | none |

With damage on, a full frame is the same into the intermediate texture, plus submit 2 of the partial frame (surface clear + 2400x1600 blit): 16 command buffers, 12 render passes.

Seen here for later phases: every presented frame with damage on clears and copies the whole 2400x1600 surface; the depth texture is stored (15 MB written) though nothing reads it after the frame; each upload is its own render pass; wgpu adds an empty Pre Pass per pass and an empty PendingWrites submit per frame.
