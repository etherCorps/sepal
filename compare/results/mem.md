# Resource comparison: RAM, GPU, CPU, size

Apple M4 Pro (Mac16,11), Version 27.0.1 (Build 26A434), 5120x2880 display at 75 Hz, AC Power. The Tracker app, window 1200x800, issues from the shared fixtures. Sessions: mem-1000, mem-10000 (2026-11-23T14:40 UTC). 74 measured runs, 0 aborted and retried. Cells: **median (min–max) of 3 runs** after 1 discarded warm-up.

Sepal is at commit 9375147 (the memory work: small atlas pages, shrinking layer textures, uploads in render passes). **† gpuix-svelte is a port** (its own widgets, a CSS subset, a pre-release Svelte 5.57.2); Tauri runs the same components and CSS as Sepal.

**What is measured**
- **RAM**: physical footprint (`footprint`, what Activity Monitor calls Memory) summed over every process the app owns: for Tauri its WebKit WebContent, GPU and Networking processes. Two macOS helpers every app gets are left out. "Peak" is each process's lifetime peak, summed.
- **GPU memory**: the part of RAM that is GPU memory: footprint's "graphics" categories (driver-owned GPU memory) and IOSurface (window surfaces), same processes. macOS reclaims most of it 1-2 s after the last frame, so it depends on whether frames are being drawn.
- **CPU**: CPU time of the same processes during the scenario, from the kernel (`proc_pid_rusage`).
- **GPU time**: time the GPU spent on the same processes' Metal command queues during the scenario: the driver's per-client `accumulatedGPUTime` (`ioreg`, AGXDeviceUserClient), which is what Activity Monitor's GPU column reads.
- Scenarios start 5 s after the app reports its list mounted. Idle and clock windows are 20 s here (CPU and GPU are also given per minute).

## App size

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| `.app` on disk | 25.3 MB | 64.7 MB | 10.8 MB | 109.2 MB |
| zipped (`ditto -c -k`) | 10.6 MB | 24.5 MB | 3.9 MB | 48.2 MB |

Tauri is small because the web engine is the system's WKWebView, not in the app; Sepal V8 carries V8 (~40 MB); gpuix carries the Bun runtime.

## 1,000 issues

### idle (nothing on screen changes) (20 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 107 (107–107) | 117 (117–117) | 144 (144–144) | 107 (107–109) |
| RAM at the end, MB | 101 (101–140) | 111 (111–111) | 132 (132–132) | 103 (103–105) |
| RAM peak, MB | 254 (254–277) | 263 (263–264) | 294 (294–294) | 225 (224–226) |
| … of it GPU memory at the end, MB | 43 (43–74) | 43 (43–43) | 6 (6–6) | 37 (37–37) |
| CPU time, ms | 4 (2–55) | 2 (2–4) | 9 (7–10) | 263 (256–268) |
| CPU, % of one core (average) | 0.0 (0.0–0.3) | 0.0 (0.0–0.0) | 0.0 (0.0–0.1) | 1.3 (1.3–1.3) |
| GPU time, ms | 0 (0–4) | 0 (0–0) | 0 (0–0) | 0 (0–0) |
| wake-ups per minute | 54 (45–210) | 42 (39–48) | 268 (261–276) | 6,132 (6,129–6,165) |

### a clock ticking once a second (20 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 262 (261–264) | 278 (271–278) | — | 222 (222–222) |
| RAM at the end, MB | 262 (260–266) | 272 (270–275) | — | 222 (222–223) |
| RAM peak, MB | 271 (270–272) | 281 (280–282) | — | 225 (224–225) |
| … of it GPU memory at the end, MB | 196 (194–198) | 196 (194–197) | — | 151 (151–151) |
| CPU time, ms | 173 (167–179) | 176 (169–182) | — | 484 (479–490) |
| CPU, % of one core (average) | 0.9 (0.8–0.9) | 0.9 (0.8–0.9) | — | 2.4 (2.4–2.5) |
| GPU time, ms | 29 (29–31) | 32 (30–37) | — | 14 (13–15) |
| wake-ups per minute | 213 (195–228) | 204 (201–219) | — | 6,294 (6,288–6,345) |

### create an issue (New issue dialog) (1 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 108 (108–108) | 117 (116–117) | — | 108 (107–108) |
| RAM at the end, MB | 358 (353–358) | 367 (363–367) | — | 245 (244–245) |
| RAM peak, MB | 363 (355–363) | 367 (366–372) | — | 245 (244–245) |
| … of it GPU memory at the end, MB | 276 (272–276) | 274 (272–274) | — | 166 (166–166) |
| CPU time, ms | 283 (278–298) | 309 (306–321) | — | 350 (346–354) |
| CPU, % of one core (average) | 20.4 (20.0–21.5) | 22.4 (22.0–23.0) | — | 24.8 (24.6–24.9) |
| GPU time, ms | 103 (98–108) | 91 (88–104) | — | 36 (33–38) |

### Show all, then wheel-scroll the whole list (9 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 108 (107–123) | 116 (116–117) | — | 108 (108–108) |
| RAM at the end, MB | 142 (141–146) | 146 (145–149) | — | 263 (262–264) |
| RAM peak, MB | 305 (304–305) | 304 (303–307) | — | 275 (273–275) |
| … of it GPU memory at the end, MB | 61 (60–62) | 60 (60–62) | — | 173 (171–173) |
| CPU time, ms | 2,491 (2,478–2,494) | 2,368 (2,366–2,373) | — | 5,291 (5,194–5,352) |
| CPU, % of one core (average) | 26.5 (26.4–26.6) | 25.2 (25.2–25.3) | — | 56.3 (55.3–57.0) |
| GPU time, ms | 583 (561–591) | 610 (596–616) | — | 482 (461–499) |

### switch views: Board, Insights, Settings, Issues (8 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 108 (107–122) | 118 (117–131) | — | 107 (107–108) |
| RAM at the end, MB | 159 (159–161) | 180 (176–183) | — | 199 (197–202) |
| RAM peak, MB | 301 (300–303) | 322 (317–324) | — | 314 (312–317) |
| … of it GPU memory at the end, MB | 59 (59–61) | 61 (59–61) | — | 62 (62–62) |
| CPU time, ms | 499 (487–501) | 451 (440–452) | — | 559 (540–633) |
| CPU, % of one core (average) | 6.1 (5.9–6.1) | 5.5 (5.3–5.5) | — | 6.8 (6.6–7.7) |
| GPU time, ms | 97 (86–107) | 97 (96–100) | — | 14 (12–14) |

## 10,000 issues

### idle (nothing on screen changes) (20 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 123 (123–125) | 136 (134–151) | — | 113 (112–113) |
| RAM at the end, MB | 117 (117–120) | 130 (128–145) | — | 106 (105–106) |
| RAM peak, MB | 271 (270–272) | 283 (281–290) | — | 232 (232–232) |
| … of it GPU memory at the end, MB | 43 (43–43) | 43 (43–58) | — | 37 (37–37) |
| CPU time, ms | 2 (2–7) | 2 (2–6) | — | 464 (461–469) |
| CPU, % of one core (average) | 0.0 (0.0–0.0) | 0.0 (0.0–0.0) | — | 2.3 (2.3–2.3) |
| GPU time, ms | 0 (0–0) | 0 (0–0) | — | 0 (0–0) |
| wake-ups per minute | 45 (42–48) | 42 (39–48) | — | 6,126 (6,120–6,165) |

### a clock ticking once a second (20 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 285 (278–286) | 288 (288–288) | — | 229 (228–230) |
| RAM at the end, MB | 282 (278–284) | 291 (288–291) | — | 226 (225–227) |
| RAM peak, MB | 287 (286–290) | 297 (297–300) | — | 233 (231–233) |
| … of it GPU memory at the end, MB | 197 (196–198) | 195 (195–196) | — | 151 (151–151) |
| CPU time, ms | 238 (234–238) | 235 (227–236) | — | 771 (754–778) |
| CPU, % of one core (average) | 1.2 (1.2–1.2) | 1.2 (1.1–1.2) | — | 3.9 (3.8–3.9) |
| GPU time, ms | 29 (29–38) | 35 (32–41) | — | 20 (16–20) |
| wake-ups per minute | 243 (216–258) | 222 (222–249) | — | 6,351 (6,327–6,438) |

### Show all, then wheel-scroll the whole list (63 s)

| | Sepal (QuickJS) | Sepal (V8) | Tauri 2 | gpuix-svelte † |
|---|---|---|---|---|
| RAM 5 s after load, MB | 123 (123–140) | 134 (134–151) | — | 113 (112–113) |
| RAM at the end, MB | 176 (176–177) | 172 (172–173) | — | 263 (262–264) |
| RAM peak, MB | 340 (337–341) | 340 (338–341) | — | 277 (273–279) |
| … of it GPU memory at the end, MB | 61 (61–61) | 61 (61–61) | — | 173 (173–173) |
| CPU time, ms | 16,779 (16,721–16,788) | 16,124 (16,116–16,450) | — | 42,532 (42,231–42,603) |
| CPU, % of one core (average) | 26.5 (26.4–26.5) | 25.4 (25.4–25.9) | — | 67.1 (66.6–67.2) |
| GPU time, ms | 3,648 (3,644–3,651) | 3,690 (3,661–3,691) | — | 3,626 (3,592–3,670) |

## Where the RAM is (clock ticking, 1,000 issues, one run, MB at the end; GPU memory in brackets)

- **Sepal (QuickJS)**: Tracker 260 (194)
- **Sepal (V8)**: Tracker 270 (194)
- **gpuix-svelte †**: tracker 222 (151)

## Aborted runs (all retried)

None.

