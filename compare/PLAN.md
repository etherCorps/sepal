# Runtime comparison: Phase 1 plan

23 Nov 2026. Sepal (ours) vs Tauri 2 vs gpuix-svelte, same Tracker app,
this Mac (M4 Pro, macOS 27.0.1, 5K display at 75 Hz, 2x). Nothing is built yet.
Sources for the Tauri and gpuix facts are at the end.

## 1. The Tracker today (`examples/tracker`, commit d3d4b07)

| | |
|---|---|
| Screens | Issues list (+ detail pane `/issues/[key]`), My issues, Project (`/projects/[key]`), Board, Insights, Settings; overlays: command palette, New issue, shortcuts, toasts; 404s |
| Routing | Sepal's built-in file router: `+layout`, `+page`, `+page.ts` load, `+error`, params |
| UI | Tailwind v4, shadcn-svelte on bits-ui (Button, Badge, Checkbox, Dialog, Input, Label, Select, Separator, Switch, Tabs, Tooltip), Lucide icons, inline SVG glyphs and an SVG bar chart |
| Fonts | `@font-face`: Inter Variable, Space Grotesk, JetBrains Mono (latin subsets, inlined) |
| `dom: true` | Yes: bits-ui needs the DOM layer |
| Desktop APIs | `fs` (readTextFile / writeTextFile / remove in the app data dir: edits.json, settings.json), `dialog.save` (Export CSV, Settings), `clipboard.writeText` (Copy link), `window.setTitle` |
| Data | Generated in JS from a seed (mulberry32, seed 42), 5,000 by default; edits saved as an overlay |
| List | Virtualised (rows in view + 300 px), paged 200 at a time, ~21 nodes per row |
| Window | 1280x800 |

## 2. What the two other runtimes are (from their current docs)

**Tauri 2** (2.12.x). Rust core process + the system WKWebView. JS runs in
WebKit's JIT inside `com.apple.WebKit.WebContent`, with `Networking` and
often `GPU` helper processes. Those helpers are XPC services whose parent is
launchd, not the app: they must be found by responsible PID / display name
("Tracker Web Content"). rAF is capped near 60 fps by a WebKit default; at
75 Hz that is 75 fps. `performance.now()` is 1 ms-granular in WKWebView.
Full DOM + CSS: the Svelte code and Tailwind run unchanged.

**gpuix-svelte** (0.6.0, experimental, one maintainer). Svelte custom
renderer for Zed's GPUI via `@gpuix/native` 0.11.0 (napi addon), running in
Node or Bun; the release build is a single Bun-compiled `.app` (~110 MB)
with the addon inside, one process. Bundles its own Svelte 5.57.2 (an
unmerged custom-renderer PR build). Frame loop ticks every 8 ms even when
idle (their notes: ~2 % CPU). No CSS engine: a property-by-property subset.

## 3. What ports, what cannot be matched

| Piece | Sepal | Tauri 2 | gpuix-svelte |
|---|---|---|---|
| Svelte components | as is | **same files** | **rewritten**: no bits-ui/shadcn, no `bind:`, no DOM |
| Tailwind v4 | as is | **same CSS** | **not supported**: hand-written class rules from the same tokens |
| bits-ui Select / Dialog / Switch / Tabs / Tooltip | as is | same | `gpuix-svelte-ui` Select / Dialog / Toggle / Tabs; no Tooltip (drop) |
| Inline SVG (Lucide, glyphs, chart) | as is | same | `<svg source="…markup…">` strings (same paths, rebuilt as strings) |
| File router | built in | **same route files**; router made DOM-side by a ~5-line shim (only 2 lines tie it to our renderer) | `gpuix-svelte-ui` `Router` with an explicit route map to the same pages; `load()` called by hand |
| Virtual list | app code | same app code | **same algorithm** in app code (spacers + visible slice from `onscroll`), not `<virtual-list>`, which still creates every row |
| fs in app data dir | `sepal/fs` | `@tauri-apps/plugin-fs` (AppData) | Node/Bun `fs` |
| Save dialog (Export CSV) | yes | `@tauri-apps/plugin-dialog` | **not documented** → not in any scenario; recorded as "not supported" |
| Clipboard (Copy link) | yes | `plugin-clipboard-manager` | **no API** → same; not in any scenario |
| Window title | yes | core window API | `set_window_title` |
| Fonts | `@font-face` | `@font-face` | `register_font(bytes)` with the same font files |
| Box shadows, transforms, transitions, z-index, `display:none`, descendant selectors | yes | yes | **missing**: Dialog centring by translate, toast/Select shadows, hover transitions, overlay stacking differ; listed per element in PORTING.md |

So: **Tauri is the same app** (same components, CSS, routes) with four
desktop calls swapped. **gpuix is a port**: same screens, data, layout,
algorithms and ids; different widget library and styling engine. Its
results will be reported with that caveat everywhere they appear.

**Keeping them equivalent**
- Structure: same routes/screens, same windowing algorithm, same paging (200).
- Node count: measured in each runtime after mount and per scenario
  (Sepal `live_nodes`, Tauri `document.getElementsByTagName('*')`
  + text nodes, gpuix renderer count if exposed) and reported, not assumed.
- Visible result: a screenshot of each screen per runtime, side by side, in
  the report.

## 4. Changes to the Tracker itself (identical in all three; go in PORTING.md)

1. **Window 1200x800** (today 1280x800).
2. **Fixture files instead of the seed generator**: `compare/shared/fixtures/issues-1000.json`
   and `issues-10000.json`, generated once from the existing `seed.ts` (so the
   data is the same as today's), with comments and activity included. The
   harness copies the chosen fixture into each app's data directory before
   launch; each app reads it with its own fs API at start (Sepal `fs`,
   Tauri plugin-fs, Node `fs`). Same file, same parse.
3. **`bench.json`** in the same data directory: `{ fixture, clock, marks }`. Read at start.
4. **A clock** (`HH:MM:SS`, one 1 s `setInterval`) in the header, shown only when
   `bench.json` says `clock: true` (scenario 4; hidden for scenario 3).
5. **`mark(name)`**: one shared module. It writes `MARK <name> <wall-clock ms>`
   (`performance.timeOrigin + performance.now()`) as one line:
   Sepal → `console.log` (host stderr), Tauri → `invoke('mark')` to a
   10-line Rust command that writes to stderr, gpuix → `process.stderr`.
   Marks at: app start, data loaded, first mount, and after each scenario's
   state change; plus "frame presented" from each runtime's own signal
   (Sepal: `SEPAL_TRACE` lines; Tauri: double `requestAnimationFrame`;
   gpuix: `next_frame()`). These are three different clocks and will be labelled so.
6. **Control positions for the harness**: at startup and after each navigation,
   the app emits the rect of every `data-testid` element the scenarios use
   (`MARK rect <id> x y w h`), so the harness clicks by id, not by hard-coded
   coordinates, in all three.

## 5. Harness

- **Input**: a small Swift tool (`compare/harness/drive`) posting real
  CGEvents: mouse move/click, key down/up, pixel wheel events at a fixed rate;
  window resize through the Accessibility API (`AXSize`).
- **Outside measurements**, the same for all three:
  - CPU % and wake-ups: `top -l` sampling `cpu`, `idlew` (and `power`) per PID.
  - Memory: `footprint -p` over **every** PID the app owns, in one call:
    Sepal: 1; gpuix: 1 (release); Tauri: app + WebContent + Networking
    (+ GPU), found by responsible PID / display name.
  - Energy: `powermetrics --samplers tasks` (energy impact per task), summed
    over the same PIDs. Needs `sudo` (see "Asks").
  - Sizes: `.app` on disk (`du -sk`) and zipped (`ditto -c -k`).
- **Frame counting**: one run per scenario recorded at 75 Hz with
  ScreenCaptureKit (Swift), frames counted by pixel change in the window.
- **Orchestrator**: a Node script (`compare/harness/run.mjs`) that launches,
  drives, samples, cools down 30 s, and writes raw data per run.

## 6. Scenarios and run rules

As you listed: 11 scenarios × 2 sizes (1,000 and 10,000) × 4 columns
(Sepal QuickJS, Sepal V8, Tauri, gpuix), 1 discarded warm-up +
10 runs each, 30 s cool-down, median / p95 / worst, raw data kept.

**Time**: with two 60 s idle scenarios and 30 s cool-downs this is roughly
**9–10 hours** of wall time (idle scenarios alone: 2 × 2 × 4 × 11 × 90 s ≈ 4.4 h).
Plan to run Phase 4 overnight, unattended.

## 7. Layout

```
compare/
  PLAN.md            this file
  PORTING.md         every difference between the three, and why
  shared/            fixtures/, bench config, mark shim, clock, fixture generator
  tauri/             Tauri 2 project; Vite root = examples/tracker/src (no copied
                     components), aliases swap the 4 desktop modules + router entry
  gpuix/             the gpuix-svelte port
  harness/           drive (Swift: CGEvent, AX resize, capture), run.mjs, metrics
  results/           <date>.json, raw/, README.md
```

## 8. Versions to pin

| | |
|---|---|
| Sepal | this repo at the commit we start Phase 2 from (now d3d4b07); QuickJS via rquickjs 0.14.0; V8 via `v8` 152.2.0; blitz-dom 0.3.0-beta.4 (13 patches) |
| App deps (Sepal and Tauri) | svelte 5.59.1, vite 8.3.8, @sveltejs/vite-plugin-svelte 7.3.1, tailwindcss + @tailwindcss/vite 4.3.5, bits-ui 2.21.0, @lucide/svelte 1.77.0, typescript 6.0.3 |
| Tauri | tauri 2.12.7, tauri-build 2.7.6, @tauri-apps/cli 2.12.6, @tauri-apps/api 2.12.5, plugin-fs 2.7.1, plugin-dialog 2.8.4, plugin-clipboard-manager 2.5.0 (wry 0.57.3, tao 0.37.3) |
| gpuix | gpuix-svelte 0.6.0, gpuix-svelte-ui 0.6.0, @gpuix/native 0.11.0 (bundles Svelte 5.57.2 PR build) |
| Toolchain | Rust 1.95.0, Node 26.13.1, Bun 1.4.5, Xcode 27.0 |

Builds: Sepal `--release` host bundled `.app` (QuickJS, and V8 with
`--features engine-v8`); Tauri `tauri build --bundles app` with stock settings
(no release-profile tuning, no Vite `build.target` change); gpuix `build()`
with defaults (no `aot`/`trim`, which need a Bun canary).

## 9. Asks before Phase 2

1. **Installs**: all project-local (npm/pnpm packages, cargo crates under
   `compare/`). Nothing system-wide. OK?
2. **Permissions** for the terminal that runs the harness (System Settings →
   Privacy & Security): **Accessibility** (to post CGEvents and resize via AX)
   and **Screen Recording** (frame counting). Without them input is silently
   dropped and capture fails (Screen Recording is already missing: I tried).
3. **`sudo`** for `powermetrics` (energy impact). Alternative without sudo:
   `top`'s `power` column only, reported as such.
4. **Run time**: ~9–10 h for Phase 4 as specified. Run it overnight, or shorten
   the cool-down for short scenarios?

## Sources

- Tauri: v2.tauri.app (create-project, frontend/vite, plugin/file-system,
  plugin/dialog, plugin/clipboard, reference/config, security/capabilities,
  concept/process-model, distribute/macos-application-bundle), crates.io and
  npm registry for versions; WebKit source (AnimationFrameRate.cpp,
  UnifiedWebPreferences.yaml, Performance.cpp, WebProcessCocoa.mm);
  tauri-apps/tauri#13978, #5889.
- gpuix-svelte: github.com/khromov/gpuix-svelte (README, docs/styling.md,
  guide.md, components.md, shipping.md, ui.md, CLAUDE.md), npm tarballs of
  gpuix-svelte 0.6.0 and @gpuix/native 0.11.0, github.com/remorses/gpuix.
