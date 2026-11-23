# Runtime comparison: quick-1

176 runs ok, 13 aborted (all retried; see Flaky). 1,000 issues. Mac16,11 (Apple M4 Pro), Version 27.0.1 (Build 26A434), display 5120x2880 at 75 Hz, AC Power, Stage Manager on. Window content 1200x800.

**Clocks.** Input events and every app line are stamped by the harness with `mach_absolute_time` (0.04 µs), app lines on arrival over stderr. App-side clocks are 1 ms. Treat latency differences under 2 ms as noise. "→ frame" uses each runtime's own signal (svelte-blitz: host repaint trace; Tauri: double rAF, mark over async IPC; gpuix: `next_frame()`), three different things. "Screen" is the independent check: first changed window frame in a 75 Hz ScreenCaptureKit recording, one recorded run per scenario, so ±13.3 ms (one frame) and a single value, not a median.

**Energy.** powermetrics was not running during this pass (its log ends 16:55, the pass ran 17:16-18:30), so energy is **top's `power` column** (Apple's unitless "energy impact" score, summed over the app's processes, mean over 1 s samples). It is a relative score, not joules.

## 1-2. Start-up (ms from exec)

### Scenario 1: first launch after quit (no reboot or cache purge, so not a cold start)

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| window on screen | 279.5 / 293.3 | 254.6 / 265.7 | 231.9 / 237.6 | 328.4 / 337.9 |
| `start` mark | 81.1 / 82.6 | 64.0 / 64.4 | 305.6 / 311.4 | 44.5 / 45.5 |
| `read` mark | 252.4 / 266.4 | 260.5 / 275.2 | 351.0 / 359.5 | 346.1 / 349.9 |
| `parsed` mark | 253.7 / 267.7 | 260.9 / 275.6 | 351.7 / 360.0 | 346.4 / 350.3 |
| `loaded` mark | 254.4 / 268.3 | 261.5 / 276.3 | 352.4 / 360.6 | 346.8 / 350.7 |
| `mounted` mark | 262.3 / 276.5 | 265.8 / 280.4 | 358.3 / 366.3 | 354.0 / 357.9 |
| first frame after `mounted` | 276.6 / 298.4 | 274.6 / 291.3 | 377.0 / 382.9 | 375.7 / 379.9 |
| first `j` accepted (ms from exec) | 414.9 / 425.2 | 401.0 / 420.0 | 393.1 / 400.0 | 490.6 / 496.3 |
| … latency of that press (ms) | 6.7 / 6.7 | 16.7 / 21.8 | 13.1 / 14.0 | 4.6 / 4.6 |
| … presses refused/ignored before it | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 1 |
| memory 5 s after `mounted` (MB, app only) | 213 / 214 | 234 / 243 | 148 / 148 | 121 / 122 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

### Scenario 2: warm start (relaunch right after quitting)

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| window on screen | 247.1 / 248.6 | 226.2 / 226.4 | 227.1 / 241.3 | 324.6 / 326.1 |
| `start` mark | 72.5 / 73.2 | 59.0 / 59.3 | 245.9 / 256.8 | 40.1 / 40.6 |
| `read` mark | 264.8 / 279.9 | 239.0 / 239.4 | 344.0 / 355.3 | 331.0 / 335.1 |
| `parsed` mark | 266.1 / 281.2 | 239.4 / 239.8 | 344.4 / 355.8 | 331.3 / 335.4 |
| `loaded` mark | 266.7 / 281.9 | 240.1 / 240.5 | 345.0 / 356.4 | 331.7 / 335.8 |
| `mounted` mark | 274.4 / 289.7 | 244.1 / 244.6 | 349.8 / 361.2 | 339.1 / 343.1 |
| first frame after `mounted` | 289.3 / 299.9 | 270.7 / 270.9 | 393.5 / 405.4 | 364.8 / 369.4 |
| first `j` accepted (ms from exec) | 386.0 / 387.0 | 382.0 / 382.8 | 411.1 / 423.4 | 489.1 / 497.6 |
| … latency of that press (ms) | 14.6 / 14.9 | 7.3 / 7.4 | 14.8 / 15.1 | 11.2 / 11.8 |
| … presses refused/ignored before it | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 1 |
| memory 5 s after `mounted` (MB, app only) | 213 / 214 | 233 / 243 | 148 / 149 | 121 / 121 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

## 5-8, 10, 11. Interactions (ms from the input event that makes the control act)

Clicks act on the mouse **release**, keys on the key **press**, in all four builds (calibrated per build and control).

### Scenario 5: open an issue, then go back

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| click row → mark | 19.3 / 20.1 | 11.6 / 13.6 | 11.1 / 11.2 | 35.4 / 35.9 |
| click row → frame | 33.9 / 37.8 | 22.1 / 24.2 | 24.9 / 26.0 | 36.7 / 37.0 |
| Escape → mark | 20.1 / 22.1 | 17.9 / 20.7 | 24.3 / 24.8 | 56.5 / 57.9 |
| Escape → frame | 37.4 / 41.8 | 37.2 / 42.8 | 50.1 / 50.2 | 57.4 / 59.4 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| screen: first change / biggest change in 500 ms (1 recorded run, ±13.3 ms) | 51 · 51 / 64 | 51 · 33 / 47 | 22 / 64 · 48 | 60 · 64 |
|---|---|---|---|---|

### Scenario 7: change an issue's status

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| click row → mark | 19.5 / 20.0 | 10.3 / 10.8 | 12.5 / 14.1 | 35.7 / 38.4 |
| click row → frame | 31.7 / 33.3 | 21.0 / 21.4 | 32.0 / 36.5 | 36.9 / 39.7 |
| click done → mark | 11.5 / 13.1 | 7.1 / 7.4 | 12.6 / 16.4 | 33.7 / 34.0 |
| click done → frame | 20.7 / 23.3 | 16.8 / 18.6 | 32.6 / 37.9 | 42.5 / 45.6 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| screen: first change / biggest change in 500 ms (1 recorded run, ±13.3 ms) | 43 / 57 · 39 | 34 / 48 · 25 | 64 · 34 | 51 / 65 · 60 |
|---|---|---|---|---|

### Scenario 8: create an issue (Cmd+Enter submits)

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| Cmd+Enter → mark | 25.2 / 26.8 | 30.3 / 37.4 | 15.8 / 21.0 | 68.9 / 72.2 |
| Cmd+Enter → frame | 45.9 / 46.3 | 51.6 / 53.0 | 36.7 / 37.8 | 74.8 / 78.1 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| screen: first change / biggest change in 500 ms (1 recorded run, ±13.3 ms) | 61 / 116 | 68 / 82 | 44 / 72 | 91 |
|---|---|---|---|---|

### Scenario 10: switch views, 2 s apart

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| click nav-board → mark | 72.1 / 73.9 | 18.3 / 19.9 | 22.7 / 25.1 | 79.6 / 87.1 |
| click nav-board → frame | 106.8 / 108.3 | 48.2 / 50.9 | 60.9 / 63.1 | 104.9 / 111.9 |
| click nav-insights → mark | 38.0 / 41.5 | 18.6 / 24.9 | 33.8 / 34.7 | 85.4 / 86.0 |
| click nav-insights → frame | 50.1 / 53.3 | 33.2 / 36.7 | 49.2 / 50.6 | 96.5 / 96.7 |
| click nav-settings → mark | 29.0 / 31.4 | 17.1 / 20.4 | 16.9 / 18.2 | 32.3 / 35.9 |
| click nav-settings → frame | 37.7 / 41.8 | 32.4 / 34.8 | 26.1 / 27.4 | 33.4 / 36.9 |
| click nav-issues → mark | 34.2 / 35.0 | 15.2 / 15.7 | 21.3 / 23.6 | 34.5 / 37.2 |
| click nav-issues → frame | 54.1 / 58.5 | 42.8 / 43.7 | 40.7 / 44.9 | 41.8 / 44.0 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| screen: first change / biggest change in 500 ms (1 recorded run, ±13.3 ms) | 12 / 122 · 11 / 67 · 22 / 37 · 11 / 55 | 11 / 80 · 37 / 50 · 16 / 44 · 18 / 62 | 19 / 76 · 5 / 47 · 1 / 40 · 2 / 56 | 123 · 59 / 113 · 46 · 57 |
|---|---|---|---|---|

### Scenario 11: resize through the Accessibility API (from the AX call)

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| resize 800x600 → mark | 16.1 / 16.9 | 19.3 / 21.3 | 6.9 / 8.1 | 25.4 / 25.4 |
| resize 800x600 → frame | 25.5 / 26.9 | 24.2 / 26.3 | 27.6 / 27.9 | 30.6 / 30.7 |
| resize 1200x800 → mark | 16.7 / 17.1 | 14.1 / 14.3 | 9.9 / 15.8 | 28.4 / 30.3 |
| resize 1200x800 → frame | 37.3 / 45.1 | 30.2 / 30.3 | 28.5 / 28.8 | 39.0 / 40.2 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| screen: first change / biggest change in 500 ms (1 recorded run, ±13.3 ms) | 29 · 36 / 55 | 41 · 34 | 29 · 17 / 49 | 68 · 43 |
|---|---|---|---|---|

### Scenario 6: type `deploy cac` into search, one key every 100 ms (all 10 keys of every run pooled)

| ms | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| key → `search:` mark | 5.2 / 14.4 | 4.0 / 8.6 | 5.0 / 17.6 | 18.8 / 40.9 |
| key → frame | 16.3 / 26.7 | 14.3 / 21.7 | 15.1 / 26.6 | 19.1 / 41.3 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.* *Pooled: 2 runs × 10 keys = 20 values per cell.*

## 3, 4, 9. Idle and scrolling

### Scenario 3: idle 60 s, nothing animating

|  | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| CPU time (ms per 60 s, all processes) | 6.5 / 6.8 | 27.5 / 48.5 | 15.0 / 15.5 | 772.0 / 823.0 |
| wake-ups (idle + interrupt, 60 s) | 34 / 34 | 36 / 40 | 172 / 173 | 5,608 / 5,611 |
| top power score (mean, unitless) | 0.01 / 0.01 | 0.03 / 0.05 | 0.01 / 0.01 | 1.25 / 1.35 |
| memory at end (MB, app only) | 178 / 178 | 196 / 203 | 132 / 132 | 113 / 113 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| frames that changed on screen (recorded run, of 75 Hz) | 2 of 4175 | 2 of 4153 | 13 of 4168 | 2 of 4145 |
|---|---|---|---|---|

### Scenario 4: idle 60 s, a clock ticking once a second

|  | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| CPU time (ms per 60 s, all processes) | 536.4 / 540.6 | 599.3 / 630.7 | 728.6 / 738.3 | 1,430.3 / 1,430.5 |
| wake-ups (idle + interrupt, 60 s) | 178 / 183 | 204 / 215 | 3,738 / 3,817 | 5,786 / 5,795 |
| top power score (mean, unitless) | 0.85 / 0.85 | 0.96 / 1.01 | 1.09 / 1.10 | 2.37 / 2.37 |
| memory at end (MB, app only) | 453 / 463 | 456 / 466 | 271 / 271 | 232 / 232 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| frames that changed on screen (recorded run, of 75 Hz) | 61 of 4005 | 60 of 4166 | 72 of 4150 | 62 of 3852 |
|---|---|---|---|---|

### Scenario 9: Show all, then wheel-scroll the whole list (391 wheel events at 60 Hz, 100 px each)

|  | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| Show all → `shown:all` (ms) | 14.9 / 16.9 | 13.5 / 15.4 | 8.9 / 10.5 | 24.2 / 24.9 |
| Show all → frame (ms) | 23.4 / 24.0 | 25.3 / 27.2 | 27.0 / 29.0 | 30.4 / 31.1 |
| CPU time, whole scenario (ms) | 2,466 / 2,469 | 2,379 / 2,397 | 2,294 / 2,319 | 5,284 / 5,378 |
| top power score (mean) | 25.9 / 26.0 | 25.1 / 25.2 | 24.2 / 24.4 | 56.3 / 57.4 |
| memory at end (MB, app only) | 238 / 249 | 244 / 255 | 497 / 512 | 280 / 286 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

| recorded run | 177 frames changed in 6.5 s (27/s); longest gap 1010 ms; first wheel → screen 58 ms | 176 frames changed in 6.5 s (27/s); longest gap 1006 ms; first wheel → screen 65 ms | 350 frames changed in 6.5 s (53/s); longest gap 86 ms; first wheel → screen 51 ms | 268 frames changed in 6.5 s (40/s); longest gap 57 ms; first wheel → screen 63 ms |
|---|---|---|---|---|

Counted from the first wheel event to 100 ms after the last. 60 wheel events/s on a 75 Hz display: a renderer that draws every event changes ~60 frames/s; the longest gap is the worst stall on screen.

## Memory (MB, app only, median)

Physical footprint over every process the app owns, minus two macOS helpers every app gets. GPU memory is reclaimed ~1-2 s after the last frame, so "at end" depends on how long ago the last frame was (scenario 3: a minute; 4: a second).

| scenario | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| 1. first launch after quit (5 s after mounted → end) | 213 → 223 | 234 → 243 | 148 → 164 | 121 → 124 |
| 2. warm start (5 s after mounted → end) | 213 → 223 | 233 → 242 | 148 → 149 | 121 → 123 |
| 3. idle 60 s, no clock (5 s after mounted → end) | 196 → 178 | 206 → 196 | 144 → 132 | 119 → 113 |
| 4. idle 60 s, clock (5 s after mounted → end) | 455 → 453 | 464 → 456 | 278 → 271 | 234 → 232 |
| 5. open and close an issue (5 s after mounted → end) | 208 → 456 | 206 → 454 | 146 → 328 | 119 → 253 |
| 6. search (5 s after mounted → end) | 204 → 445 | 206 → 455 | 148 → 337 | 120 → 254 |
| 7. change status (5 s after mounted → end) | 214 → 476 | 206 → 480 | 148 → 338 | 119 → 253 |
| 8. create an issue (5 s after mounted → end) | 214 → 533 | 217 → 547 | 148 → 380 | 119 → 255 |
| 9. show all, then scroll top to bottom (5 s after mounted → end) | 207 → 238 | 217 → 244 | 146 → 497 | 118 → 280 |
| 10. navigate four screens (5 s after mounted → end) | 212 → 244 | 206 → 263 | 146 → 215 | 119 → 210 |
| 11. resize (5 s after mounted → end) | 211 → 324 | 206 → 224 | 146 → 296 | 119 → 182 |

## CPU time per scenario (ms, all processes, median)

| scenario | svelte-blitz QuickJS | svelte-blitz V8 | Tauri 2 | gpuix † (Svelte 5.57.2) |
|---|---|---|---|---|
| 1. first launch after quit | 382 / 384 | 396 / 398 | 546 / 547 | 492 / 496 |
| 2. warm start | 389 / 389 | 393 / 395 | 549 / 553 | 505 / 512 |
| 3. idle 60 s, no clock | 7 / 7 | 27 / 48 | 15 / 16 | 772 / 823 |
| 4. idle 60 s, clock | 536 / 541 | 599 / 631 | 729 / 738 | 1,430 / 1,431 |
| 5. open and close an issue | 83 / 83 | 85 / 86 | 104 / 107 | 121 / 121 |
| 6. search | 188 / 190 | 194 / 199 | 251 / 257 | 213 / 214 |
| 7. change status | 209 / 212 | 231 / 236 | 170 / 171 | 168 / 172 |
| 8. create an issue | 266 / 269 | 286 / 295 | 246 / 247 | 346 / 348 |
| 9. show all, then scroll top to bottom | 2,466 / 2,469 | 2,379 / 2,397 | 2,294 / 2,319 | 5,284 / 5,378 |
| 10. navigate four screens | 511 / 513 | 478 / 479 | 415 / 419 | 532 / 541 |
| 11. resize | 50 / 51 | 58 / 59 | 72 / 73 | 83 / 83 |

*Cells: median / worst of 2 measured runs (1 warm-up and the screen-recorded run excluded); with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.* *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*

## Flaky and aborted

| run | attempt | reason |
|---|---|---|
| blitz-1000-s11-r1 (recorded) | 1 | AX resize failed: -25200 |
| blitz-1000-s11-r1 (recorded) | 2 | AX resize failed: -25200 |
| blitz-1000-s11-r1 (recorded) | 3 | AX resize failed: -25200 |
| blitzv8-1000-s11-r1 (recorded) | 1 | AX resize failed: -25200 |
| blitzv8-1000-s11-r1 (recorded) | 2 | AX resize failed: -25200 |
| blitzv8-1000-s11-r1 (recorded) | 3 | AX resize failed: -25200 |
| tauri-1000-s4-r1 (recorded) | 1 | window moved or resized: {"alpha":1,"h":115,"id":8125,"name":"Issues · Tracker","onscreen":true,"w":103,"x":-307,"y":1112} vs {"alpha":1,"h":832,"id":8125,"name |
| tauri-1000-s11-r1 (recorded) | 1 | AX resize failed: -25200 |
| tauri-1000-s11-r1 (recorded) | 2 | AX resize failed: -25200 |
| tauri-1000-s11-r1 (recorded) | 3 | AX resize failed: -25200 |
| gpuix-1000-s11-r1 (recorded) | 1 | AX resize failed: -25200 |
| gpuix-1000-s11-r1 (recorded) | 2 | AX resize failed: -25200 |
| gpuix-1000-s11-r1 (recorded) | 3 | AX resize failed: -25200 |

**Latencies whose worst run is over 1.5× the median** (medians over 5 ms):

- Tauri 2 s11 `resize 1200x800`: median 9.9, worst 15.8 ms

## What went wrong, and what was changed

- **Resize scenario (11), recorded run, all four builds: 12 aborts, `AX resize failed: -25200`.** Harness bug, not the apps. While a window is screen-captured, macOS adds a 66x20 AX window to the app and lists it first; `drive resize` took the first AX window and the system refused to resize it. Fixed (`drive.swift`: the largest AX window), verified on blitz and Tauri with recording on, and the four recorded runs re-run on resume (attempt 4, all ok). `run.mjs` got `--attempts` for that resume. No measured number changed.
- **Tauri scenario 4, recorded run, attempt 1: aborted**, Stage Manager shrank the window to a 103x115 thumbnail (focus lost). Attempt 2 ok. Stage Manager stays on (it cannot be turned off on this Mac); a run it touches aborts and is retried, never measured.
- **The screen-recorded run is excluded from all statistics.** In start-up runs the capture starts before the harness polls for the window, adding ~280 ms to "window on screen" and to the first input in every build; capture also adds ~10-20 MB and a few % CPU. Interaction latencies moved < 1 ms. It now only feeds the screen check. So the quick pass statistics are over **2 runs**. For the full pass: 1 warm-up + 10 measured + 1 recorded.
- **svelte-blitz scrolls 2× as far per wheel event as Tauri and gpuix (a svelte-blitz bug, measured and found in code).** winit turns a pixel wheel delta into *physical* pixels (×2 on this display, `winit-appkit view.rs` `scrollWheel:`) and Blitz applies it as CSS px (`blitz-dom events/pointer.rs` `handle_wheel`). In scenario 9 svelte-blitz reached the end of the list ~3.0 s into the 6.5 s wheel; Tauri and gpuix at ~6.0 s. Not fixed (no changes to svelte-blitz during the comparison). Consequences: scenario 9 is **not like for like** for svelte-blitz: half the scrolling work, then ~3 s of wheel events at the end of the list; its scroll CPU and frame counts are not comparable as they stand.
- **Energy:** powermetrics was not running (log ends 16:55), so the top power score is used, labelled as such.
- **Screen check:** "first change" is often a press/hover restyle (e.g. a nav link losing its pressed state 1-20 ms after release); "biggest change in 500 ms" is the result, and agrees with each runtime's own frame signal within a frame or two.

