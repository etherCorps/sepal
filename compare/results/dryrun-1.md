# Runtime comparison: dry run (dryrun-1)

**Status: dry run, not results.** One run per scenario, no warm-up, 1,000 issues only, no screen recording. Its purpose was to prove the harness end to end. Single runs carry no statistics: do not draw conclusions from differences of a few ms, and do not treat any of this as the comparison.

## Setup

| | |
|---|---|
| Machine | Mac16,11, Apple M4 Pro, 14 cores, 64 GB |
| macOS | Version 27.0.1 (Build 26A434) |
| Display | 5120x2880 px at 2560x1440 pt (2x), 75 Hz |
| Power | AC Power; Low Power Mode off; Stage Manager **on** (see failures) |
| Started | 2026-11-23T09:40:29.356Z (UTC) |
| App | the Tracker (Linear-style issue tracker), 1,000 issues from a shared fixture file, window content 1200x800 |

### Builds

| Build | Runtime | JS engine | Svelte | Versions | .app on disk | zipped |
|---|---|---|---|---|---|---|
| svelte-blitz QuickJS | Blitz (Rust HTML/CSS engine), native window | QuickJS 0.14.0 (rquickjs, interpreter) | 5.59.1 | blitz-dom 0.3.0-beta.4 (13 patches) | 25.2 MB | 10.6 MB |
| svelte-blitz V8 | same host, built with `engine-v8` | V8 (crate 152.2.0, JIT) | 5.59.1 | same | 64.6 MB | 24.5 MB |
| Tauri 2 | system WKWebView | JavaScriptCore (JIT), in WebKit's WebContent process | 5.59.1 | tauri 2.12.7, wry 0.57.3, tao 0.37.3 | 10.8 MB | 3.9 MB |
| gpuix-svelte † | GPUI (Zed's UI framework) via @gpuix/native 0.11.0 | Bun 1.4.5 (JavaScriptCore, JIT) | **5.57.2 (pre-release custom-renderer PR build)** | gpuix-svelte 0.6.0 | 109.2 MB | 48.2 MB |

svelte-blitz and Tauri run the **same Svelte components and the same Tailwind CSS** (byte-identical app.css); only four desktop calls and the router's DOM hook differ. **† gpuix-svelte is a port**: same screens, data, store logic and list-windowing algorithm, but its own widgets (no bits-ui/shadcn), a CSS subset instead of Tailwind, icons as SVG strings, and a pre-release Svelte 5.57.2. Every gpuix number carries that caveat.

## How it was measured

- **Input**: real OS-level events (CGEvent: mouse, keys, pixel wheel) from a Swift tool, posted only while the app is frontmost; window resize through the Accessibility API.
- **Clock**: one monotonic clock (`mach_absolute_time`, ~0.04 µs) stamps the input events, every line the app writes, and (later) captured frames. App marks (`MARK <name>`) are stamped on arrival on stderr.
- **Latency** = OS input event → the app's mark after its DOM/state update ("→ mark"), and → the runtime's own "frame" signal ("→ frame"), **from the event that makes the control act**. Calibration launches made after the dry run (each control pressed once and held 300 ms) found the same in all four builds: every clicked control acts on the **release** (mouse up), every key on the **press** (key down). The dry run measured clicks from the press, which added the click's hold (a 30 ms sleep, 32.7 ms when stamped later) to every click; it did not stamp the release. Click rows below are therefore **estimates from the release: as measured minus the median hold (≈, ±3 ms: stamped holds ranged 30-35 ms)**, with the as-measured value from the press in brackets. Key rows are unchanged. The three frame signals are **different clocks of different things**: svelte-blitz = host repaint trace (`RedrawRequested`); Tauri = double `requestAnimationFrame` (mark sent over async IPC, ~1.1-1.5 ms later; rAF ≤ 75 fps here); gpuix = `next_frame()` (GPUI's next 8 ms tick, not presentation). The independent check (screen recording at 75 Hz) was **not** run in this dry run.
- **App clock resolution**: 1 ms in all four (svelte-blitz `Date.now()`, WKWebView `performance.now()`); arrival stamps are ~0.04 µs. Treat differences under ~2 ms, and under one 75 Hz frame (13.3 ms) for anything frame-related, as noise.
- **CPU, wake-ups**: `proc_pid_rusage` at scenario start and end, summed over every process the app owns (found by responsible PID: Tauri's WebKit WebContent/GPU/Networking helpers included). No energy figure: see the note under scenario 9.
- **Memory**: `footprint` over the same processes in one call (physical footprint, as Activity Monitor's "Memory"). Totals include two macOS helpers attributed to every app (`SetStoreUpdateService`, `SafariPlatformSupport.Helper`); the app-only figure is shown separately.
- "First launch after quit" (not "cold start": no reboot or file-cache purge).

## 1-2. Start-up (ms from exec, one run each)

### Scenario 1: first launch after quit

| Event | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| window on screen | 276.5 | 241.2 | 205.5 | 312.5 |
| `start` (first app code ran) | 84.2 | 68.0 | 297.2 | 52.8 |
| `loaded` (fixture read + parsed) | 231.7 | 246.9 | 349.8 | 341.3 |
| `mounted` (list rendered) | 240.0 | 250.9 | 354.9 | 348.5 |
| first frame after `mounted` | 247.3 | 273.2 | 373.6 | 371.6 |
| first `j` posted by the harness | 407.8 | 587.9 | 376.6 | 429.7 |
| first input accepted (`highlight` mark) | 414.9 | 594.6 | 395.9 | 468.3 |
| **first `j` → `highlight`** (the app's part) | **7.1** | **6.7** | **19.3** | **38.7** |
| `j` presses posted before the mark / highlights in the log | 1 / 1 | 1 / 1 | 1 / 1 | 2 / 2 |

### Scenario 2: warm start (relaunch right after quitting)

| Event | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| window on screen | 247.8 | 226.8 | 213.0 | 322.7 |
| `start` (first app code ran) | 68.4 | 58.5 | 248.8 | 47.1 |
| `loaded` (fixture read + parsed) | 247.2 | 259.2 | 346.7 | 333.2 |
| `mounted` (list rendered) | 255.1 | 263.4 | 351.0 | 340.4 |
| first frame after `mounted` | 276.6 | 273.8 | 395.9 | 363.4 |
| first `j` posted by the harness | 382.6 | 600.8 | 398.1 | 444.0 |
| first input accepted (`highlight` mark) | 396.4 | 608.1 | 412.3 | 492.7 |
| **first `j` → `highlight`** (the app's part) | **13.8** | **7.3** | **14.2** | **48.7** |
| `j` presses posted before the mark / highlights in the log | 1 / 1 | 1 / 1 | 1 / 1 | 3 / 3 |

"First input accepted" from exec is mostly the harness: it posts the first `j` only once the window is settled (bounds unchanged over 5 polls, 100+ ms after it appears) and frontmost, then every 20 ms until a highlight mark arrives. The app's part is the first `j` → `highlight` row. In every build every press posted was accepted (as many highlight marks, one row further each, as presses): gpuix's "2-3 presses" are presses posted while its first was still being handled (it marks a key in ~35-50 ms, the harness re-presses every 20 ms). svelte-blitz V8's ~600 ms was the harness posting late (588 / 601 ms after exec; V8 marked the press 7 ms later), during the minutes when Stage Manager was moving windows (see "What went wrong"). Re-run on 23 Nov, scenarios 1-2 twice per build (`raw/firstinput/`): at every press the app was active and frontmost, and (asked through AX in one set) had a focused, main window; no press was dropped; first `j` → `highlight` was 18-23 ms (QuickJS), 17-23 (V8), 20-28 (Tauri), 38-49 (gpuix). The first key a fresh process gets costs more than later ones in every build (svelte-blitz handles the second in ~2 ms). `read` / `parsed` marks were added after the dry run.

## 5-8, 10, 11. Interactions (ms, one run each)

Each cell: **→ mark / → frame** (see "How it was measured" for what each means and its resolution). Clicks: ≈ from the release (estimate), in brackets as measured from the press. Keys: from the key press.

### Scenario 5: open an issue, then go back

| Step | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| click row | ≈23.2 / ≈33.5 (55.9 / 66.2) | ≈7.4 / ≈18.8 (40.1 / 51.5) | ≈10.5 / ≈27.0 (43.1 / 59.7) | ≈27.4 / ≈28.5 (60.0 / 61.2) |
| Escape | 21.9 / 33.0 | 10.4 / 19.8 | 23.5 / 47.2 | 63.2 / 65.4 |

### Scenario 7: change an issue's status

| Step | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| click row | ≈26.6 / ≈49.8 (59.3 / 82.4) | ≈13.1 / ≈26.9 (45.7 / 59.6) | ≈17.8 / ≈37.4 (50.5 / 70.1) | ≈38.0 / ≈39.2 (70.7 / 71.9) |
| click done | ≈10.4 / ≈17.4 (43.1 / 50.1) | ≈10.2 / ≈20.1 (42.9 / 52.8) | ≈16.0 / ≈38.9 (48.6 / 71.6) | ≈37.1 / ≈49.4 (69.8 / 82.0) |

### Scenario 8: create an issue

| Step | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| Cmd+Enter | 27.8 / 42.1 | 17.3 / 30.0 | 14.5 / 38.5 | 80.7 / 86.2 |

### Scenario 10: switch between the main views

| Step | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| click nav-board | ≈74.1 / ≈115.8 (106.8 / 148.5) | ≈20.7 / ≈50.9 (53.4 / 83.6) | ≈27.5 / ≈64.9 (60.2 / 97.6) | ≈78.4 / ≈103.2 (111.1 / 135.9) |
| click nav-insights | ≈32.3 / ≈44.1 (65.0 / 76.7) | ≈30.4 / ≈44.6 (63.0 / 77.3) | ≈41.2 / ≈62.1 (73.9 / 94.8) | ≈96.5 / ≈107.1 (129.2 / 139.8) |
| click nav-settings | ≈31.5 / ≈43.4 (64.2 / 76.1) | ≈17.0 / ≈29.9 (49.6 / 62.6) | ≈19.7 / ≈29.2 (52.4 / 61.9) | ≈40.5 / ≈42.6 (73.2 / 75.3) |
| click nav-issues | ≈33.9 / ≈47.5 (66.6 / 80.2) | ≈15.3 / ≈37.7 (48.0 / 70.4) | ≈20.5 / ≈41.9 (53.1 / 74.6) | ≈44.3 / ≈53.8 (77.0 / 86.5) |

### Scenario 11: resize the window

| Step | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| resize 800x600 | 14.3 / 19.0 | 13.9 / 18.8 | 5.8 / 26.1 | 24.2 / 29.0 |
| resize 1200x800 | 16.2 / 25.1 | 17.4 / 30.5 | 8.2 / 34.7 | 29.2 / 41.3 |

### Scenario 6: type `deploy cac` into search, one key every 100 ms

| Key | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| `d` | 18.9 / 37.8 | 15.8 / 28.2 | 9.9 / 29.6 | 38.3 / 38.7 |
| `e` | 7.6 / 18.3 | 4.7 / 14.1 | 4.8 / 17.0 | 31.8 / 32.0 |
| `p` | 11.7 / 19.2 | 5.7 / 13.6 | 8.9 / 24.8 | 31.2 / 31.4 |
| `l` | 8.6 / 15.3 | 3.6 / 14.1 | 11.8 / 22.6 | 32.8 / 33.0 |
| `o` | 4.1 / 15.2 | 9.6 / 20.4 | 6.1 / 23.4 | 28.6 / 28.7 |
| `y` | 4.7 / 11.3 | 9.3 / 16.9 | 5.8 / 24.0 | 32.1 / 32.3 |
| ` ` | 4.1 / 11.9 | 9.3 / 16.8 | 5.8 / 25.1 | 32.4 / 32.5 |
| `c` | 5.4 / 16.2 | 11.6 / 20.7 | 14.4 / 20.7 | 36.6 / 36.7 |
| `a` | 3.4 / 6.1 | 5.8 / 11.6 | 23.2 / 29.8 | 20.1 / 20.1 |
| `c` | 4.8 / 8.6 | 4.3 / 10.3 | 13.1 / 17.0 | 24.8 / 25.0 |
| **median** | **5.1 / 15.3** | **7.5 / 15.5** | **9.4 / 23.7** | **32.0 / 32.1** |

With 1,000 issues each keystroke filters every issue; the list is windowed (only rows in view are mounted, ~28 rows), so this mostly measures filtering plus re-rendering one screen of rows.

## 3, 4, 9. Idle and scrolling (one run each)

### Scenario 3: idle 60 s, nothing animating

| Metric | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| measured for (s) | aborted ×3 | 60.0 | 60.0 | 60.0 |
| CPU, % of one core | aborted ×3 | 0.0 % | 0.0 % | 2.2 % |
| CPU time (ms) | aborted ×3 | 8 | 30 | 1,315 |
| wake-ups per s (idle + interrupt) | aborted ×3 | 0.6 | 3.4 | 97.2 |
| memory, all processes (MB) | aborted ×3 | 190 | 135 | 118 |
| memory, app's own processes only (MB) | aborted ×3 | 176 | 133 | 105 |
| processes counted | aborted ×3 | 3 | 6 | 3 |

### Scenario 4: idle 60 s, a clock ticking once a second

| Metric | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| measured for (s) | 60.0 | aborted ×3 | 60.0 | 60.0 |
| CPU, % of one core | 0.8 % | aborted ×3 | 1.8 % | 3.7 % |
| CPU time (ms) | 488 | aborted ×3 | 1,058 | 2,203 |
| wake-ups per s (idle + interrupt) | 2.9 | aborted ×3 | 61.1 | 100.1 |
| memory, all processes (MB) | 445 | aborted ×3 | 269 | 237 |
| memory, app's own processes only (MB) | 432 | aborted ×3 | 267 | 223 |
| processes counted | 3 | aborted ×3 | 6 | 3 |

### Scenario 9: Show all, then wheel-scroll the whole list (391 wheel events at 60 Hz, 100 px each)

| Metric | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| measured for (s) | 9.4 | 9.4 | 9.4 | 9.4 |
| CPU, % of one core | 26.3 % | 27.0 % | 31.9 % | 57.1 % |
| CPU time (ms) | 2,471 | 2,538 | 2,996 | 5,368 |
| wake-ups per s (idle + interrupt) | 33.7 | 44.7 | 343.2 | 135.9 |
| memory, all processes (MB) | 231 | 233 | 521 | 276 |
| memory, app's own processes only (MB) | 218 | 220 | 520 | 263 |
| processes counted | 3 | 3 | 6 | 3 |
| wheel interval median / p95 (ms) | 16.7 / 18.4 | 16.7 / 18.5 | 16.7 / 18.4 | 16.7 / 18.4 |

Energy is not reported. The dry run recorded `ri_billed_energy` (rusage V4), which is **not** the process's energy: it is energy *other* processes spent on its behalf and billed to it through vouchers (XNU's bank ledger), so it barely moved with the app's own CPU. The harness now records `ri_energy_nj` (rusage V6, the process's own energy estimate in nJ; checked: populated, ~5.9 W for a busy P-core). It goes back in the report only once it agrees with powermetrics on at least one scenario (`compare/results/powermetrics_energy.py`).

Scenario 9 latency and smoothness come only from the screen recording, which this dry run did not do; here it shows only the cost of scrolling.

## Memory by process (MB, at the end of scenario 5)

| Process | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| `Tracker` | 430 | 463 | — | — |
| `com.apple.SafariPlatformSupport.Helper` | 12 | 12 | — | 12 |
| `SetStoreUpdateService` | 2 | 2 | 2 | 2 |
| `com.apple.WebKit.GPU` | — | — | 173 | — |
| `com.apple.WebKit.WebContent` | — | — | 113 | — |
| `tracker` | — | — | 32 | 225 |
| `com.apple.WebKit.Networking` | — | — | 6 | — |
| `com.apple.audio.SandboxHelper` | — | — | 6 | — |
| **total** | **443** | **477** | **331** | **239** |

### Memory across scenarios (MB, all processes, end of each run)

| Scenario | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| 1. first launch after quit | 452 | 453 | 313 | 252 |
| 2. warm start | 445 | 473 | 298 | 236 |
| 3. idle 60 s, no clock | aborted ×3 | 190 | 135 | 118 |
| 4. idle 60 s, clock | 445 | aborted ×3 | 269 | 237 |
| 5. open and close an issue | 443 | 477 | 331 | 239 |
| 6. search | 448 | 486 | 331 | 256 |
| 7. change status | 465 | 483 | 328 | 253 |
| 8. create an issue | 525 | 554 | 431 | 257 |
| 9. show all, then scroll top to bottom | 231 | 233 | 521 | 276 |
| 10. navigate four screens | 257 | 266 | 217 | 197 |
| 11. resize | 437 | 439 | 300 | 243 |

**Why these swing (found after the dry run):** each build owns GPU memory that macOS reclaims about 1-2 s after the app last presented a frame, and that comes back with the next frame. svelte-blitz's "Owned physical footprint (unmapped) (graphics)" goes 312 → 92 MB with the app frontmost and doing nothing (no trace event, no CPU); Tauri's WebKit GPU process (~120 MB of graphics) shrinks or leaves; gpuix's graphics go 151 → 37 MB. Scenario 3 (no clock) ends a minute after the last frame, so it shows the low state; scenario 4's clock repaints every second, so it shows the high one; scenario 10 (2 s after the last click) and 9 (measured a few seconds after scrolling stopped) show the low one, the others the high one. Not a leak, not the app freeing memory: see "Memory over time" below. A memory figure means something only with "how long since the last frame" next to it.

## CPU time per scenario (ms of CPU, all processes)

| Scenario | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| 1. first launch after quit | 389 | 391 | 562 | 500 |
| 2. warm start | 374 | 404 | 543 | 517 |
| 3. idle 60 s, no clock | aborted ×3 | 8 | 30 | 1,315 |
| 4. idle 60 s, clock | 488 | aborted ×3 | 1,058 | 2,203 |
| 5. open and close an issue | 85 | 74 | 106 | 139 |
| 6. search | 183 | 300 | 300 | 265 |
| 7. change status | 209 | 240 | 202 | 199 |
| 8. create an issue | 280 | 347 | 261 | 381 |
| 9. show all, then scroll top to bottom | 2,471 | 2,538 | 2,996 | 5,368 |
| 10. navigate four screens | 496 | 709 | 499 | 655 |
| 11. resize | 51 | 61 | 85 | 108 |

## Added after the dry run (23 Nov, separate launches, not part of dryrun-1)

### Memory over time, left alone after `mounted` (MB, all processes; `run.mjs memwatch`)

F = the app was frontmost at that sample. Focus moved to Zed, Finder and Chrome during these runs (logged), which is why some samples are not frontmost.

| Build | clock | +0 s | +5 s | +10 s | +15 s | +30 s | +60 s |
|---|---|---|---|---|---|---|---|
| svelte-blitz QuickJS | no | 428 F | 474 F | 454 | 232 | 218 | 210 |
| svelte-blitz QuickJS | yes | 429 F | 448 F | 463 F | 454 F | 453 | 212 |
| svelte-blitz V8 | no | 440 F | 208 F | 208 F | 190 F | 190 F | 455 F |
| svelte-blitz V8 | yes | 441 F | 457 F | 455 F | 453 F | 229 | 204 |
| Tauri 2 | no | 262 F | 155 | 153 | 140 | 146 | 140 |
| Tauri 2 | yes | 272 F | 270 | 269 | 265 | 143 | 143 |
| gpuix-svelte † | no | 226 F | 121 F | 121 F | 145 | 272 F | 149 |
| gpuix-svelte † | yes | 231 F | 122 | 122 | 117 | 117 | 120 |

Sampled every second (svelte-blitz V8, frontmost throughout, last repaint at +0.05 s, no trace event after it): GPU memory +0 s 314, +1 s 312, +2 s 92, +3 s 92 MB, then flat.

### What the memory is (MB; `run.mjs breakdown`: footprint categories and `vmmap --summary`, 5 s after `mounted` and after scenario 9)

| Build | when | app's processes | GPU (Metal/IOSurface) | malloc heaps | JS engine regions | kernel-owned, unmapped | other (binaries, stacks, page tables...) |
|---|---|---|---|---|---|---|---|
| svelte-blitz QuickJS | mounted + 5 s | 185 | 109 | 50 | 0 | 18 | 9 |
| svelte-blitz QuickJS | after scenario 9 | 217 | 126 | 64 | 0 | 18 | 9 |
| svelte-blitz V8 | mounted + 5 s | 195 | 109 | 42 | 14 | 18 | 11 |
| svelte-blitz V8 | after scenario 9 | 222 | 126 | 57 | 9 | 18 | 12 |
| Tauri 2 | mounted + 5 s | 273 | 123 | 60 | 1 | 9 | 79 |
| Tauri 2 | after scenario 9 | 467 | 125 | 125 | 2 | 8 | 207 |
| gpuix-svelte † | mounted + 5 s | 107 | 37 | 34 | 22 | 4 | 10 |
| gpuix-svelte † | after scenario 9 | 273 | 174 | 41 | 40 | 8 | 10 |

Both svelte-blitz captures are in the low GPU state (a few seconds after the last frame); in the high state add ~220 MB of GPU memory (memwatch above). gpuix is in its low state at start-up (37 MB GPU) and its high one after scenario 9 (174 MB). Tauri's GPU process held ~120 MB of graphics at both points. svelte-blitz QuickJS at ~430 MB is therefore: ~310-350 MB GPU memory (Metal resources owned by the process, mostly "unmapped (graphics)", plus 15-30 MB of IOSurfaces, its window's drawables), ~45-65 MB malloc (QuickJS's heap, which uses malloc, together with Blitz's DOM, Stylo and the Rust side: footprint cannot split them), 18 MB kernel-owned unmapped memory, ~10 MB binaries and stacks. V8 is the same plus ~10-15 MB of V8 heap regions (VM tag 16), with less malloc. Tauri: the WebContent process (JS and DOM: WebKit Malloc, JSC regions) grows from ~94 to ~280 MB over scenario 9; its GPU process holds ~120 MB graphics; the app process ~31 MB. gpuix: one process, Bun/JSC heap in tagged regions (20-40 MB), malloc ~30 MB, GPU 37-174 MB.

### 10,000 issues (one launch each, bench mode, `run.mjs probe <build> 10000 5`; ms from exec)

| Event | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| `start` | 70.4 | 52.9 | 223.4 | 32.2 |
| `read` | 243.5 | 212.8 | 314.3 | 306.4 |
| `parsed` | 260.2 | 216.8 | 318.5 | 311.0 |
| `loaded` | 268.6 | 219.6 | 320.5 | 312.7 |
| `mounted` | 288.2 | 227.4 | 328.3 | 322.4 |
| memory 5 s after `mounted` (MB, all processes) | 222 | 227 | 289 | 256 |

No build takes seconds or gigabytes with 10,000 issues. Tauri's window was parked by Stage Manager (it launched behind Chrome) and was activated by the probe before its memory sample.

### One clock tick in svelte-blitz (scenario 4 on the diagnostic builds; `diag_ticks.py`, ms per tick, median / max of ~60)

| Phase | QuickJS | V8 |
|---|---|---|
| ops applied | 1.00 / 1.00 | 1.00 / 6.00 |
| layout the app forced (it measures): total | 0.24 / 0.46 | 0.24 / 0.54 |
| … style | 0.01 / 0.03 | 0.01 / 0.05 |
| … box construction | 0.07 / 0.13 | 0.07 / 0.17 |
| … layout | 0.15 / 0.32 | 0.14 / 0.30 |
| … paint tree | 0.00 / 0.01 | 0.00 / 0.01 |
| repaint (`RedrawRequested`: Blitz paint + vello + present) | 7.50 / 22.90 | 7.80 / 17.90 |
| CPU per tick, all threads (rusage / ticks) | 7.57 | 7.63 |
| wake-ups per second | 2.9 | 3.3 |

A tick costs ~7.5 ms of CPU, almost all of it the repaint of the whole window for one changed text node; style + layout of the change is ~0.25 ms. The trace cannot split: the JavaScript of the turn (timer callback, Svelte's effects; with CPU ≈ the timed phases it is at most ~1 ms), the repaint into Blitz's scene building vs vello's rendering vs waiting for a drawable / presenting, the delay between the timer's deadline and the wake-up, and the time from present to the screen. The diagnostic hosts are never used for measured runs.

## What went wrong

| Build | Scenario | Attempts | Outcome | Reason |
|---|---|---|---|---|
| svelte-blitz QuickJS | 3 | 3 | no valid run | window moved/shrunk by Stage Manager after focus left the app (09:42:09, 09:42:17, 09:43:24 UTC) |
| svelte-blitz V8 | 3 | 2 | ok on attempt 2 | window moved/shrunk by Stage Manager after focus left the app (09:47:31 UTC) |
| svelte-blitz V8 | 4 | 3 | no valid run | window moved/shrunk by Stage Manager after focus left the app (09:49:46, 09:49:54, 09:51:03 UTC) |
| svelte-blitz V8 | 7 | 2 | ok on attempt 2 | window moved/shrunk by Stage Manager after focus left the app (09:51:28 UTC) |

42 of 44 build×scenario pairs produced a valid run; 8 attempts were aborted and none was measured. All aborts happened between 09:42 and 09:51 UTC while the person running it and the assistant were active in another app (Zed); no aborts in the 22 runs after that. No crashes, no app errors in any log, no missed input.

## Not yet done (before the real passes)

- Screen recording at 75 Hz (the independent frame/latency check, and the only smoothness measure for scrolling).
- powermetrics energy impact (the user runs it; matched to runs by timestamp).
- 10,000-issue fixture, warm-ups, and 3 (quick) / 10 (full) runs per scenario with median, p95 and worst.
- Stage Manager off; nothing else touching the Mac during a pass.
