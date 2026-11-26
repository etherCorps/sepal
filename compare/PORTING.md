# PORTING: how the four builds differ, and why

Every difference between the builds of the Tracker used in the comparison.
Section 1 applies to all of them (changes made to the Tracker itself to make it
measurable). Sections 2-4 are per runtime, written while porting. The shared
contract is `shared/PROTOCOL.md`.

## Summary

| | Sepal QuickJS / V8 | Tauri 2 | gpuix-svelte |
|---|---|---|---|
| Svelte source | examples/tracker | **the same files** | a port: same screens, data, store logic (copied), windowing algorithm, ids |
| Svelte version | 5.59.1 | 5.59.1 | **5.57.2, a pre-release PR build bundled by gpuix** |
| CSS | Tailwind v4 | **byte-identical stylesheet** (SvelteKit's `bundle.*.css`) | hand-written classes in gpuix's CSS subset |
| Widgets | shadcn-svelte / bits-ui | same | gpuix-svelte-ui (Select, Dialog, Toggle, Tabs) |
| Icons, chart | inline SVG | same | `<svg source>` strings; chart as 3 stacked SVGs |
| Router | SvelteKit 3.4.0, a single-page app in one bundle (since 24 Nov; Sepal's built-in file router before) | **the same SvelteKit install**, with adapter-static | gpuix-svelte-ui Router with an explicit map |
| Fonts | @font-face (woff2) | @font-face (same CSS) | register_font with the same fonts converted to TTF; **Inter only at weight 400** |
| Viewport | 1200x800 | 1200x800 (frame 1200x832) | 1200x800 |
| JS engine | QuickJS (interpreter) / V8 | WebKit JSC (JIT) in WebContent | Bun (JSC, JIT) |
| Processes | 1 (+2 system helpers) | app + WebContent + GPU + Networking (+2) | 1 |
| "After re-render" marks | Svelte `tick()` | `tick()` | `Promise.resolve().then(flushSync)` (gpuix's async mode makes `tick()` throw) |
| Frame signal | host trace `RedrawRequested` | rAF→rAF (≤75 fps here, rAF capped near 60 on >60 Hz displays) | `next_frame()` (GPUI's next 8 ms tick, not presentation) |
| Mark latency | stderr write | async IPC (~1.1-1.5 ms) | stderr write |
| App clock | Date.now() (1 ms) | performance.now() (1 ms in WKWebView) | performance.now() |
| Save dialog, clipboard | yes | yes | **not supported** (no API): Export CSV and Copy link show an error toast; no scenario uses them |
| Idle | event-driven | event-driven (WebKit) | **8 ms frame loop runs while idle (by design)** |

Known layout difference at 1200 px with the detail pane open: the list toolbar
overflows by ~75 px in Sepal and Tauri (same CSS); in gpuix the Select
widgets are wider (min 180 px) so search shrinks and Group/Sort clip. None of
the controls the scenarios use is covered.


## 1. Changes to the Tracker (all builds) and the Sepal builds


What was added to `examples/tracker` for `compare/shared/PROTOCOL.md`. These
changes are in the shared app source, so they apply to the Tauri build too
(same files); the gpuix port has to reproduce them. Nothing in Sepal
itself (packages/, crates/) was changed.


`bench.json` in the app's data directory
(`~/Library/Application Support/<app id>/`). `src/lib/bench.ts` reads it at
start with `readTextFile('bench.json', { dir: 'data' })`; if it is missing or
unreadable the app is the normal Tracker and nothing below prints or changes.
If it is there, `fixture.json` from the same directory is read and parsed
(a missing fixture in bench mode throws: the harness's mistake).

### Changes to the app

| File | Change |
|---|---|
| `sepal.config.ts` | window 1280x800 -> **1200x800** (all builds, not only bench) |
| `src/lib/bench.ts` (new) | `bench` (null until known), `bench_ready` (promise), `mark(name)`, `rects()`; emits the `start` mark when evaluated |
| `src/lib/store.svelte.ts` | first import is `./bench` (so `start` is the first app code to run); startup waits for `load()` **and** `bench_ready`; in bench mode it replaces the issues with the fixture (`use_base`, the old body of `reseed`) and ignores edits.json / settings.json (settings stay the defaults: no simulated teammates, group by status); marks `loaded`, `mounted`, `status:<key>:<status>` (in `bulk_update`), `created:<key>` (in `create`) |
| `src/routes/+layout.svelte` | marks `nav:<path>` (`afterNavigate`, once per change of path; an effect on the path before 24 Nov), `search:<query>` (effect on `view.search`), `resize:<w>x<h>` (window `resize` event), the clock |
| `compare/shared/make-fixtures.ts` (new) | writes `fixtures/issues-1000.json` and `issues-10000.json` from `seed(n)`, shape `{ n, issues }` |

Marks are written after the DOM update: each one waits for Svelte's `tick()`
after the state change. The clock: when `bench.json` has `clock: true`, a
`bench-clock` span (`HH:MM:SS`, local time) appears next to "Tracker" at the top of the
sidebar (the shell has no other header; each screen draws its own), updated
by one 1 s `setInterval` that marks `clock` per tick. With `clock: false` there
is no element and no interval.

RECT lines (`rects: true` only): after `mounted`, after each navigation, and
400 ms after every `pointerdown` / `keydown` (which is what opens a Dialog or
Select; 400 ms lets it settle). Every element with a protocol `data-testid`
that has a non-empty box is reported.

### Where this differs from PROTOCOL.md

1. **Line prefix.** Sepal's host prints `console.log` to **stdout** as
   `[js] ...`, and `console.error` to stderr as `[js error] ...`. bench.ts
   calls the host's log at level 2 directly, so lines arrive on **stderr** as
   `[js error] MARK start 1795425748260.0`. The harness should match
   `(MARK|RECT) .*` anywhere in the line. (Not `console.error`: on V8 the
   global `console` is V8's own, which prints nothing without an inspector,
   so `console.*` output is lost on the V8 host. That is a Sepal bug,
   left alone here.)
2. **`start` arrives late.** Whether bench mode is on is only known once
   bench.json was read, which is a host call answered on a later turn (and
   held until the window exists). `start` is recorded at the first statement
   and written then, with its original `app_ms`. Use `app_ms` for `start`,
   not the arrival time. All other marks are written when they happen.
3. **The seed still runs, and is shown first.** fs is async in every runtime,
   so the store makes and renders the usual 5,000 seeded issues at start (as
   without bench.json), and replaces them with the fixture at `loaded`.
   `start -> loaded` therefore includes `seed(5000)` and the first render of
   it; `mounted` is the list rendered **with the fixture** (`tick()` after
   `loaded`; the route is already `/issues` by then). The first frame the
   window paints shows the seed's list, not the fixture's.
4. **`app_ms`** is `performance.timeOrigin + performance.now()` only where
   `timeOrigin` is a wall clock. QuickJS-ng's `timeOrigin` is a monotonic
   clock and the V8 host has none, so both use `Date.now()` (1 ms).
5. **Names with spaces.** `search:<query>` contains the query as typed, so
   `MARK search:deploy cac 1795...` has a space in the name: `app_ms` is the
   last field.
6. **RECT names beyond the plain ids.** Rows: `RECT row:<key> ...` (every
   mounted row, which includes ~300 px below the window). Select items:
   `RECT status-option-<value> ...` for every open Select's items (the gpuix
   name; only the status Select is opened in the scenarios).
7. **RECT and transforms.** Sepal's `getBoundingClientRect` leaves CSS
   transforms out. Select content is placed by an inline
   `transform: translate(Xpx, Ypx)`; bench.ts adds that back (only on
   Sepal, only that form). The New issue dialog has no transform, so
   its rects are right as reported. Checked against CPU screenshots.
8. **`resize:<w>x<h>`** is the content area (`innerWidth` x `innerHeight`).
   Setting the window to 800x600 through System Events' `size` (the outer
   frame) gives `resize:800x568` on this Mac (32 px title bar).
9. **Writes are not suppressed.** A status change or a new issue still writes
   edits.json 500 ms later (as the app does); only reading is ignored.
10. **No `frame:` marks** from Sepal: the harness uses the first
    `[trace] ... RedrawRequested` line after a mark (`SEPAL_TRACE=1`).

### Layout at 1200x800

Fits with the list alone (toolbar, Group/Sort, rows). **With the detail pane
open it does not**: the list column is 1200 - 240 - 480 = 480 px, the toolbar
needs ~555 px, and the Project filter runs ~75 px over the pane's top edge
(at 1280 it fit exactly). Left as is, since it is the same CSS in Tauri and
changing it is an app change; `status-select` and the Select items are not
covered by it. A fix would be `overflow-hidden` on the toolbar row or hiding
a filter while the pane is open.

### Builds (`compare/builds/`)

```sh
pnpm --filter tracker build
cargo build --profile dist --features screenshot --manifest-path crates/host/Cargo.toml
cp crates/host/target/dist/calyx compare/builds/hosts/calyx-quickjs
cargo build --profile dist --features "screenshot engine-v8" --manifest-path crates/host/Cargo.toml
cp crates/host/target/dist/calyx compare/builds/hosts/calyx-v8
compare/builds/make.sh
```

Features: `window` (default; includes blitz-paint, so inline SVG and
scrollbars draw) plus `screenshot`, as `scripts/build-host-package.js` builds
the host that ships in `@ethercorps/calyx`. The `dist` profile, as that
host is built (`crates/host/Cargo.toml`: release with fat LTO in one codegen
unit, stripped); results from before 26 Nov 2026 used `--release` (thin LTO),
which is as fast in the headless bench and a quarter bigger. `make.sh` copies
`examples/tracker/dist` with only `app.json`'s `id` changed
(`com.compare.tracker.blitz` / `.blitzv8`; the example keeps
`com.example.tracker`), runs `<host> --bundle <copy> <out>`, which writes
`Tracker.app` (CFBundleName/Executable `Tracker`, CFBundleIdentifier from
app.json), and renames it to `Tracker-blitz.app` / `Tracker-blitzv8.app`.
The data directory comes from the bundled `Resources/app.json` id.

#### Added after the first build

- **Bench mode is decided at build time** (`VITE_BENCH=1` for Sepal and Tauri, `bun build.ts --bench` for gpuix). Without it the app seeded and drew 5,000 issues before it knew it was in bench mode, which inflated start-up in Sepal and Tauri only. A bench build starts empty, marks `start` first, and loads `fixture.json`. Non-bench builds are unchanged (tests pass).
- **`highlight:<key>`** mark (ui.svelte.ts) and **`shown:all`** mark (store `show_all`), and RECTs for `show-all`/`show-more`: scenarios 1-2 (first accepted input) and 9 (scroll) need them.

#### Added after the dry run (23 Nov, before the quick pass)

- **`read` and `parsed` marks**, splitting `start -> loaded`: `read` when the
  fixture file's text has been returned (before `JSON.parse`), `parsed` right
  after `JSON.parse`. Same two lines in every build:
  `src/lib/bench.ts` (Sepal and Tauri, the same file) and
  `compare/gpuix/src/persist.ts` (`load_fixture`, through `write_mark` from
  `start.ts`). Neither gets a `frame:` mark: `compare/tauri/src/bench-emit.js`
  skips them, as gpuix's `mark()` skips `start` and `loaded` (gpuix calls
  `write_mark` directly, which never makes one). Tauri still sends
  `frame:start` / `frame:loaded`, as before; nothing reads them.
- **Diagnostic builds** (`compare/builds/make.sh`): `Tracker-blitz-diag.app`
  and `Tracker-blitzv8-diag.app` (ids `com.compare.tracker.blitzdiag` /
  `.blitzv8diag`), the same app on hosts built with
  `--features "screenshot blitz-dom/log-phase-times"` (+ `engine-v8`), which
  print blitz-dom's per-phase resolve times on stdout. Never measured; used to
  split a clock tick (scenario 4) into phases.

#### Added after the quick pass (23 Nov, before the full pass)

- **Toolbar beside the detail pane** (a bug the 1200px window caused: change 1
  above). With the pane open the list is 480px wide, the toolbar was laid out
  for 560px; it does not wrap, so the Project filter spilled 85px into the
  pane and, being in the toolbar's `relative z-10`, was drawn over it (the
  same in a browser: Tauri had it too). Now, beside the pane, the Assignee and
  Project filters show only while they filter something. Same change in
  `examples/tracker/src/lib/list/{Filters,IssueList}.svelte` (Sepal and
  Tauri) and `compare/gpuix/src/list/{Filters,IssueList}.svelte`; test
  `the_toolbar_stays_out_of_the_pane_at_the_1200px_window`. Scenarios 5 and 7
  open the pane, so they now render two fewer buttons than in the quick pass.
- **`drive resize` picks the largest AX window** (harness, not the apps):
  while a window is screen-captured macOS lists a 66x20 AX window first.

## 2. Tauri 2


Build: `cd compare/tauri && pnpm install --ignore-workspace && VITE_BENCH=1 pnpm exec tauri build --bundles app`,
then `cp -R src-tauri/target/release/bundle/macos/Tracker.app ../builds/Tracker-tauri.app`.
`beforeBuildCommand` runs `pnpm run build:web` first: `vite build` in
`examples/tracker` with `compare/tauri/vite.config.js` as its config. The
repository's `pnpm install` has to have run, for the linked toolchain below.

**Since 24 Nov (branch `sveltekit-only`) both builds run SvelteKit.** Results
from before that date ran Sepal's built-in router on both sides (its runtime
copied for the DOM, as this section used to describe).


- **Source.** No component, route or CSS file is copied. The build runs in
  `examples/tracker`, so Vite's root and SvelteKit's project are the app's:
  `src/routes`, `src/lib` (through `#lib`), `src/app.html` and `src/app.css`
  are its own files, bench mode (`src/lib/bench.ts`) included.
- **CSS: byte-identical.** SvelteKit's stylesheets in
  `_app/immutable/assets` are the same files in `compare/tauri/dist` and in
  the Sepal build's `dist` (checked with `cmp` on 24 Nov, when there was one
  stylesheet; since 25 Nov both are SvelteKit's split build and write each as
  it is). Tailwind scans the Vite root, which is why the build runs in
  `examples/tracker`.
- **Fonts.** The same `@font-face` rules with the same files (Inter Variable,
  Space Grotesk, JetBrains Mono, latin subsets), as files in
  `_app/immutable/assets` in both (data URLs before 24 Nov).
- **Bundle shape.** SvelteKit's default split build, version polling off:
  the options the Tracker's own vite.config.ts gives `sveltekit()` (since 25
  Nov; before, one bundle, as `sepalKit()` set it). Both write SvelteKit's
  output as it is: `index.html`, the shell from `src/app.html`, and the ES
  module chunks it imports, Sepal's adapter with one `<style>` added for the
  route announcer. Vite's default minification. Resolve conditions are
  `sepal()`'s for SvelteKit apps minus `svelte-custom-renderer`: `browser,
  svelte, module, import, default`.
- **Package versions.** App files import their packages from where Node
  resolution finds them, the repo's workspace install that
  `examples/tracker` uses: svelte 5.59.1, bits-ui 2.21.0, @lucide/svelte 1.77.0,
  tailwind-variants 3.3.1, cn 0.5.2, @internationalized/date 3.12.4,
  tw-animate-css 1.4.0, shadcn-svelte 1.8.1, @fontsource* 5.3.0. These are the same
  files, not just the same versions. The toolchain is the app's too:
  `vite build` runs from `examples/tracker`, and `compare/tauri/package.json`
  links `@sveltejs/kit` and `@tailwindcss/vite` from
  `examples/tracker/node_modules`, so vite 8.3.8, @sveltejs/kit 3.4.0,
  @sveltejs/vite-plugin-svelte 7.3.1 (its compiler is svelte 5.59.1),
  tailwindcss and @tailwindcss/vite 4.3.5 are the Sepal build's installs.
  Only `@sveltejs/adapter-static` 4.0.1 is Tauri's own. One Svelte runtime and
  one SvelteKit runtime in the bundle (the same counts of their error codes as
  in the Sepal bundle).
- **Router.** SvelteKit's client router, from the one install both builds use:
  `$app/navigation`, `$app/state`, `afterNavigate`, `error` and `redirect`
  from `@sveltejs/kit`, link clicks heard on `<html>`. Both builds take the
  same unknown-address route (`src/routes/[...path]`, a 404). In both, a link
  click waits a `requestAnimationFrame` and a `setTimeout(0)` before
  navigating (SvelteKit lets the page repaint first).
- **Entry.** SvelteKit's client entry
  (`@sveltejs/kit/src/runtime/client/entry.js`), with
  `compare/tauri/src/bench-emit.js` imported first, where `sepal()` puts the
  DOM layer and the runtime.
- **Aliases:** only `sepal/{fs,dialog,clipboard,window}`.

### Different, on purpose

| What | Sepal | Tauri | Why |
|---|---|---|---|
| Svelte output | custom renderer (`customRenderer`), op buffer to Blitz | Svelte's normal DOM client | that is the comparison |
| `bind:` on elements | rewritten to attachments by `bindings.js` | compiled by Svelte | Svelte supports it in the DOM; the preprocessor exists only because Svelte refuses it for custom renderers |
| `<svelte:head>` preprocessor | `head.js` | not used | the app has no `<svelte:head>` |
| DOM layer | `sepal/dom` loaded first (`sepal()` always loads it in a SvelteKit app) | none | WebKit is a DOM |
| Events | `dispatch()` in `renderer.js` bubbles | WebKit's events; Svelte delegates `onclick` and similar to the mount target | DOM build behaviour. SvelteKit's click listener on `<html>` runs after the app's handlers in both |
| `sepal/{fs,dialog,clipboard,window}` | host calls | `compare/tauri/src/desktop.js` on the plugins | see below |
| HTML page | SvelteKit's `index.html`, from `src/app.html`, as `sepal/adapter` writes it: the stylesheet, `modulepreload`s of the chunks, and SvelteKit's start script in `<body>`, which the host runs | adapter-static's `index.html`, from the same `src/app.html` | the same page, but for the route announcer's `<style>` |
| `app.json`, `icon.png` | written; the icon is the Sepal one, since the Tracker has none of its own | not used; the icon is in `src-tauri/icons` (made from `assets/brand/sepal-app-icon-1024.png` with `tauri icon`, the icon the Tracker gets from the preset) | Tauri's equivalent is `tauri.conf.json` |
| App id / data dir | `com.compare.tracker.blitz` | `com.compare.tracker.tauri` (`~/Library/Application Support/com.compare.tracker.tauri`) | PROTOCOL.md |

#### Desktop shims (`compare/tauri/src/desktop.js`)

Only what the Tracker imports, with Sepal's names and signatures:

- `readTextFile`, `writeTextFile`, `remove`: `@tauri-apps/plugin-fs`; `{ dir: 'data' }`
  becomes `{ baseDir: BaseDirectory.AppData }` (on macOS the same
  `~/Library/Application Support/<id>`). Other `dir` names throw: the app uses none.
  Sepal creates missing parents on write and plugin-fs does not, so the
  Rust side creates the app data directory at startup (`create_dir_all` in `setup`).
- `save`: `@tauri-apps/plugin-dialog`. Sepal's `defaultPath` is a directory
  and `defaultName` the file name; Tauri has one `defaultPath`, so they are joined.
  The picked path is in plugin-fs's scope because the dialog plugin adds it.
- `writeText`: `@tauri-apps/plugin-clipboard-manager`.
- `setTitle`: `getCurrentWindow().setTitle()` (`@tauri-apps/api/window`). Checked:
  the window is titled "Issues · Tracker" after start.

Capabilities (`src-tauri/capabilities/default.json`): `core:default`,
`core:window:allow-set-title`, `fs:default`, `fs:allow-appdata-read-recursive`,
`fs:allow-appdata-write-recursive` (it includes `write-all`, so `remove` and
`write_text_file` too), `dialog:allow-save`, `clipboard-manager:allow-write-text`.
CSP is off (`null`), as in Tauri's templates.

#### Marks (`compare/tauri/src/bench-emit.js`)

- Imported first in SvelteKit's entry (the `tracker-tauri:bench-emit` plugin in
  `vite.config.js`), so `globalThis.__bench_emit` exists before `bench.ts`
  marks `start`. Each line goes to the `mark` command
  (`invoke('mark', { line })`), which `eprintln!`s it.
- **IPC latency.** `invoke` is asynchronous (WebKit message handler to the Rust
  core). Measured on this Mac by timestamping lines as they arrive: arrival
  minus `app_ms` was 1.1 to 1.5 ms for `loaded`, `mounted` and the `frame:`
  marks, which includes `app_ms`'s 1 ms granularity. Sepal writes
  stderr synchronously from the JS thread.
- **`start` arrives late, by design of `bench.ts`:** it is held until
  `bench.json` has been read (about 145 ms after `app_ms`). The same holds for
  Sepal. Use `app_ms` for `start`, not the arrival time.
- **`frame:<name>`** is emitted after every non-`frame` MARK line, from
  `requestAnimationFrame(() => requestAnimationFrame(…))`, as PROTOCOL.md says.
  It is WebKit's nearest "frame presented" signal: the second callback runs at
  the start of the frame after the one that drew the change, so it lands one
  to two frame intervals after the mark. Measured: `clock` to `frame:clock`
  about 15 ms, startup marks to their `frame:` 35 to 45 ms. Sepal has no `frame:`
  marks: the harness uses its trace repaint lines, a different clock.
  `frame:start` and `frame:loaded` are scheduled when the held lines are
  flushed, so they share the frame of `frame:mounted`.
- **rAF rate.** WebKit throttles rAF to about 60 fps by default
  (`PreferPageRenderingUpdatesNear60FPSEnabled`). Not changed; I did not
  measure it on this 75 Hz display. Frame marks are quantised by it.
- **`performance.now()`** is 1 ms in WKWebView, so `app_ms` values end in `.0`.
  `bench.ts` uses `performance.timeOrigin + performance.now()`, and so do the
  `frame:` marks.
- **Uncaught errors and unhandled rejections** are also sent to stderr, as
  `ERROR <stack>` lines, because the Sepal host prints its errors to stderr
  and WebKit's console is not visible otherwise. None appeared in any run.

### Things that render or behave differently (the engine)

Not adjusted. Here so that screenshots and timings can be read with them in mind:

- **Text.** WebKit shapes and rasterises with CoreText (subpixel positioning,
  macOS font smoothing). Blitz uses parley and Vello. Same font files, so
  metrics should be close but glyph edges and line breaks can differ. The
  fallback in `--font-sans` (`ui-sans-serif, system-ui`) is SF Pro in WebKit and
  whatever fontique picks in Blitz; it only matters for glyphs missing from Inter.
- **UA stylesheet.** WebKit's `html.css` and Blitz's differ under Tailwind's
  preflight, for example in form controls and focus rings.
- **CSS Blitz lacks and WebKit has.** `backdrop-filter` (`backdrop-blur-xs` on
  the Dialog overlay), container queries (`@container/card-header`),
  `overflow-y: auto` (Select content), `position: sticky`, 3D transforms. They
  take effect in Tauri only.
- **Bits UI.** In WebKit it gets a real `getComputedStyle`, real
  `animationend`/`getAnimations()` and real layout reads. Sepal's DOM layer
  answers those from a forced layout and a blank computed style, so exit
  animations (Dialog, Select) can end on different frames.
- **Processes.** Tauri runs the app process plus WebKit's `WebContent`, `GPU`
  and `Networking` XPC services (parent launchd). All three were seen running.
  JS runs in JavaScriptCore with its JIT, in `WebContent`.
- **Window size: not equal (open issue).** `width: 1200, height: 800` in
  `tauri.conf.json` gives a **1200x768** viewport here (`innerWidth`/`innerHeight`
  in the page). tao reports inner size = outer size = 2400x1600 physical, and
  System Events gives the window frame as 1200x800. So on macOS 27 Tauri's size
  includes the 32 px title bar. Sepal's 1200x800 is the content area: its
  frame is 32 px taller (checked on the running Sepal Tracker: frame
  1280x832 for a 1280x800 window). Left as the brief specified. To get the same
  1200x800 viewport, set `"height": 832`. The AX resize in scenario 11 sets
  frame sizes, so it gives both runtimes the same frame.
- **Release profile** is Cargo's default (no LTO, opt-level or strip changes) and
  Tauri's default bundle settings.

#### Added after the first build

- Window `height` 832: Tauri's size includes the 32 px title bar; Sepal's is the content area. 832 gives the same 1200x800 viewport.
- `bench-emit.js` now takes mark names with spaces (`search:deploy cac`).

## 3. gpuix-svelte


23 Nov 2026. The port is `compare/gpuix` (sources in `compare/gpuix/src`), built from
`examples/tracker` at d3d4b07. This file lists what is not the same, element by element. What is
not listed is meant to be the same: data, routes, sorting, filtering, paging, grouping, windowing,
key map, toasts, edits and their Undo.


| | Sepal Tracker | gpuix port |
|---|---|---|
| Svelte | 5.59.1 (npm), compiled by Vite with the `svelte-custom-renderer` condition | **5.57.2**, the unmerged custom-renderer PR build bundled inside gpuix-svelte (`node_modules/gpuix-svelte/node_modules/svelte`); compiled by gpuix-svelte's own Bun plugin, **with `experimental.async: true`** forced on (compile-core.js) |
| Runtime | QuickJS-ng (or V8) inside the Rust host | Bun 1.4.5 (JavaScriptCore JIT), compiled into one executable by `Bun.build({ compile })` |
| Renderer | Sepal → Blitz (Stylo CSS, Taffy layout, Vello) | gpuix-svelte 0.6.0 → @gpuix/native 0.11.0 (napi addon) → GPUI (Zed), Taffy layout, Metal |
| Widgets | bits-ui 2.21 under shadcn-svelte | gpuix-svelte-ui 0.6.0 (Select, Dialog, Toggle, Tabs, Checkbox, Menu, Router) + gpuix-svelte's Portal, dismiss layers |
| CSS | Tailwind v4 + tw-animate-css, full Stylo cascade | hand-written class rules per component, translated property by property (no CSS engine) |
| Icons | @lucide/svelte 1.77.0 components | the same Lucide 1.77.0 path data as `<svg source>` strings (`scripts/icons.ts` generated `src/icons.ts`) |
| Fonts | @fontsource woff2 via `@font-face` (inlined data URLs) | the same files, decompressed to TTF once (`scripts/fonts.ts`, wawoff2 2.0.1), embedded with Bun `with { type: 'file' }`, `register_font()` after `render()` |

### Build and bench mode

- `bun build.ts --bench` → `dist/Tracker.app` (gpuix-svelte `build()` defaults: no `aot`, no
  `trim`, unsigned), copied to `compare/builds/Tracker-gpuix.app`. 109 MB on disk (`du`:
  111,804 KB), 48 MB zipped (`ditto`). One process.
- Bench mode is a build-time flag (`src/build-flag.ts`, rewritten by `build.ts --bench` and put
  back after), like `VITE_BENCH=1` for the others. The bench build never calls `seed()`: it starts
  with no issues, `MARK start` is the first statement of the first module evaluated
  (`src/start.ts`, imported first by `standalone.ts`), then `bench.json` is read synchronously and
  `fixture.json` asynchronously (`node:fs/promises`), swapped in, `loaded`, then `mounted` after
  the flush. A bench build without `bench.json` runs with `clock:false, rects:false`.
- `mark()` writes `MARK <name> <performance.timeOrigin + performance.now()>` with
  `process.stderr.write` (Bun's `timeOrigin` is wall-clock ms).
- **`tick()` is not Svelte's.** With async mode on, Svelte's `tick()` calls
  `requestAnimationFrame`, which Bun does not have: it throws (the first bench build crashed after
  `loaded`). `src/bench.ts` uses `Promise.resolve().then(flushSync)`, which is what `tick()` does
  outside async mode, for every "after re-render" mark (`mounted`, `nav:`, `search:`, `status:`,
  `created:`, `highlight:`, `shown:all`, `resize:`, `clock`).
- **Frame signal:** `frame:<name>` follows every mark except `start` and `loaded`, from
  gpuix-svelte's `next_frame()` (imported from `gpuix-svelte/lib/render.js`, not exported by the
  package). It resolves after the frame loop's next `native.tick()` following the commit, which is
  GPUI's frame pump, not a vsync/presentation timestamp.
- `nav:<path>` comes from the router's `after` hook (skipping the first screen); `search:<query>`
  from an effect on `view.search` in the shell; `resize:<w>x<h>` from `onlayout` on the root
  element (the window's content box), not a resize event; the clock is in the sidebar's top row,
  as in the Sepal Tracker.
- RECT lines: a walk of gpuix-svelte's shadow tree from the root element, boxes from
  `getElementBounds(nativeId)` (last painted frame, window content coordinates, logical px).
  Emitted after `mounted` and every other mark's frame, plus a 250 ms poll that prints only what
  changed (calibration launch only: `rects:true`). Rows are `row:<key>` (and `row` for the first
  one inside the list's box); the Select's options, which gpuix-svelte-ui names
  `status-select-<value>`, are printed as `status-option-<value>`.
- Window: `render(App, { width: 1200, height: 800 })` sizes the **content area**: `RECT list`
  ends at y = 92 + 708 = 800.

### Runtime behaviour that differs regardless of the app

- **Frame loop:** gpuix-svelte ticks GPUI every 8 ms (`FRAME_MS`) whenever the window is active,
  even idle; `top` showed ~1.3 % CPU for the idle bench app a few seconds after start. Sepal
  has no frame loop.
- **Escape order:** gpuix-svelte runs Escape through its dismiss-layer stack after every key
  handler. The shell's Escape (close the pane, clear the selection, drop the newest toast) is a
  dismiss layer registered under every other, so open Selects and Dialogs take Escape first, as
  Bits UI's do. Same order, different mechanism.
- **Keys** arrive from `on_window_key` as GPUI key events and are turned into the
  KeyboardEvent fields the Tracker's keymap reads (`key_event()` in `src/ui.svelte.ts`); the keymap,
  chords and `combo()` are the Tracker's. "Typing" is gpuix's `e.editing`; "a layer is open" is the
  app's overlay flags plus a focused Select trigger with `aria-expanded` (the Tracker scans
  `document.body` for Bits UI layers).
- **Hit testing:** mouse events do not bubble in GPUI; every clickable box uses `hitbox="self"`.
- **Scrolling:** the list's height comes from `onlayout`, the offset from `onscrollchange`, and
  programmatic scrolling (j/k reveal, the past-the-end clamp) is `native.scrollTo()`. Same window
  arithmetic. Wheel deltas and scroll physics are GPUI's.
- **Svelte compile mode:** async mode is on in gpuix-svelte's compiler for every component (the
  Tracker compiles without it).

### Element by element

#### Shell (`App.svelte` ← `routes/+layout.svelte`)
- Root grid `240px minmax(0,1fr)` → a flex row (aside 240 px fixed, main flex 1).
- Sidebar links are `div`s with `onclick → navigate()`, not `<a href>` with a global click handler.
- No `transition-colors` on hover anywhere (GPUI has no transitions); hover colours switch at once.
- "PROJECTS" is written upper-case (no `text-transform`), and `tracking-[0.06em]` is dropped (no
  `letter-spacing`).
- Fonts: **Inter is a variable font and GPUI does not apply its weight axis**: every Inter text
  renders at the default instance (400). `font-medium`/`font-semibold` in Inter (row titles, active
  nav item, buttons, user name, headings) look regular. Space Grotesk is registered under its file's
  family name "Space Grotesk Light" (the static 500/600 instances both declare it); JetBrains Mono
  400/500 work.
- `line-height: 1.45` → 18.85 px (GPUI reads unitless line-height as px).
- Window title: `set_window_title` (same strings).

#### Routing
- gpuix-svelte-ui `Router` (memory mode) with an explicit table: `/`, `/issues`, `/issues/:key`,
  `/my`, `/projects/:key`, `/board`, `/insights`, `/settings`, `*`. `/` redirects to `/issues`
  through a `before` hook (the Tracker's `+page.ts` redirect).
- The Router remounts its screen on every path change (`{#key router.path}`). To keep the
  Tracker's behaviour, `/issues` and `/issues/:key` map to an empty screen and the list + pane
  (`IssuesLayout`) are drawn from the Router's `layout` snippet, outside the key: the list stays
  mounted and the detail pane is not remounted when the key changes (j/k with the pane open),
  as with the Tracker's router.
- **`/projects/ENG` → `/projects/WEB` remounts the list** (the Tracker keeps the page component).
  No scenario does this.
- 404s: `/issues/<unknown>` shows the same two lines as the Tracker's `issues/+error.svelte` (it is
  the pane's "deleted" branch); `/projects/<unknown>` and unknown paths show the root error page.
  No `load()` functions; the lookups happen in the screens.
- No `?week=` URL parsing by `URLSearchParams`: `router.query.week`.

#### Issues list (`list/IssueList.svelte`, `Row.svelte`)
- Windowing: unchanged algorithm (row 36/28, header 32, more-row 36, overscan 300, spacers).
- Toolbar: the four filter Selects are gpuix-svelte-ui Selects, **at least 180 px wide and 34 px
  tall** (the Tracker's are compact h-7 triggers). The toolbar does not fit: search shrinks to
  ~160 px and the Group/Sort selects are clipped off the right edge (`overflow: hidden`). With the
  pane open, Priority onwards are clipped too. Clicking them is not in any scenario.
- Search: GPUI `<input>` with its native placeholder (the Tracker overlays a span because Blitz draws
  none). Focus ring is the border colour, on keyboard focus only.
- Rows: same cells and widths, as a flex row instead of CSS grid. The checkbox is a drawn 14 px box
  with a check SVG when checked (no `<input type="checkbox">`). Title, key, labels, avatar, date as
  in the Tracker. Nodes per row are not the same as in Blitz (no comment nodes, text nodes only for
  non-blank text); not measured here.
- Pick (filter/property selects): **no glyph** in the trigger or the items (gpuix-svelte-ui Select
  takes text options only). The "Group"/"Sort" prefix and the "filter set" dot sit beside the
  trigger, not inside it. Variants become style props (`--ui-*`): ghost = transparent border and
  fill; graphite (bulk bar) also makes the **option list dark**, where the Tracker's list stays light.
- Assignee filter list: one long column of 27 options (the Tracker lays it out in two columns).
- Shift+click ranges: same logic; "keep the box ticked" needs no `preventDefault` (nothing toggles
  on its own).
- Group header: a `button` element (gpuix's focusable div), same content.

#### Bulk bar (`list/BulkBar.svelte`)
- Centred by a full-width flex row (no `calc(50% - 280px)`); no `shadow-sm`.
- Delete confirmation: gpuix-svelte-ui `Dialog` (size md, 440 px, centred, title + description +
  close button). The Tracker's has no close button; this one does (gpuix's Dialog shows it whenever
  Escape and the scrim can close it).

#### Detail pane (`detail/Detail.svelte` ← `routes/issues/[key]/+page.svelte`)
- Same header buttons. Tooltip titles (`title=`) are gone (GPUI has no native tooltips).
- **Copy link: not supported** (no clipboard API in gpuix-svelte). The button shows the Tracker's
  failure toast ("Could not copy the link. Copy the key … instead.").
- Title edit: GPUI `<input>`; Enter is `onsubmit`; the hint sits at a fixed 40 px below (no
  `top: 100%`); `-mx-2`/`w-[calc(100%+16px)]` become negative margins. No `tracking-[-0.02em]`.
- Description: no `max-w-[62ch]` (the pane is narrower than that anyway).
- Properties: a flex row per property (label 96 px) instead of a two-column grid; Selects as above
  (34 px tall where the Tracker's ghost triggers are 28 px, so the block is taller).
- Labels editor: gpuix-svelte-ui `Checkbox` (18 px box, accent fill) + dot + name, in a 3-column
  grid (`grid-template-columns: 3`).
- Thread: gpuix-svelte-ui `Tabs` (underlined tabs, 14 px medium). **Counts are part of the tab
  label** ("Comments 2"), not a separate muted mono span. Activity entries: the author's name and
  the text are two text elements in a row (GPUI text does not nest spans).
- Comment box: native placeholder; Enter is `onsubmit`.

#### Board (`board/Board.svelte`, `Card.svelte`)
- Columns get the scroller's height from `onlayout` (a percentage height under a scroll container
  resolves to the content here, and `h-full` columns collapsed to their headers).
- Nested scroll areas use `pointer-events: auto` (GPUI scrolls every scroll container under the
  pointer otherwise).
- `s` (status of the focused card): gpuix-svelte-ui **`Menu`** opened at the card's painted box
  (`position`), where the Tracker opens a hidden Bits UI Select anchored to the card. The gpuix
  Select cannot be opened from code or anchored elsewhere. The Menu has no arrow-key navigation.
- Focus ring: `outline` 2 px in the ring colour. Card title clamps with `line-clamp: 2` (the
  Tracker clips at `max-h-[2.9em]`).
- "+ N more in list" is a button that sets the filter and navigates.

#### Insights (`insights/Insights.svelte`, `Chart.svelte`)
- Range: gpuix-svelte-ui Tabs (underlined) for the Tracker's pill segmented control. Project:
  gpuix-svelte-ui Select (no project square in the trigger).
- **Tile tooltips are dropped** (gpuix-svelte-ui has no Tooltip; gpuix-svelte's Tooltip hugs its
  trigger, which stops the tiles filling their columns). No `tabular-nums`.
- Grids (2fr 1fr 1fr 1fr; 3fr 2fr; 140px 1fr 44px) → flex rows with the same proportions; no
  `mx-auto` centring (the page is narrower than its 1120 px max anyway).
- Chart: GPUI draws an `<svg>` as a one-colour mask, so the chart is **three stacked SVGs** (grid
  lines in `--border`, created bars in `--chart-1`, closed bars in `--chart-2`) with the same scale,
  ticks and bar geometry. Positions the Tracker gives in `%` are computed in px from the plot width
  (`onlayout`), and the SVG is drawn at that width instead of stretched with
  `preserveAspectRatio="none"`. Grid lines are 1 px rects rather than `<line>` with crispEdges.
  Week links are absolutely positioned divs with `onclick → navigate('/issues?week=N')`.
- Oldest open: Row as in the list; the last row keeps its bottom border (no `:last-child`).

#### Settings (`screens/Settings.svelte`)
- Tabs: gpuix-svelte-ui Tabs (underlined) for shadcn's pill tabs. Switches: gpuix-svelte-ui
  `Toggle` (38×22, knob animated by GPUI `motion`). Grouping: gpuix-svelte-ui Select. Separators:
  1 px divs. Labels are plain text (no `<label for>`).
- **Export CSV: not supported** (no save dialog in gpuix-svelte; `prompt_for_paths` opens files or
  directories only). The button shows an error toast instead of writing a file.
- Reset confirmation: gpuix-svelte-ui Dialog size sm (340 px) with a close button.
- Data is saved with Node/Bun `fs` in `~/Library/Application Support/com.compare.tracker.gpuix`
  (the Tracker uses Sepal's `fs` with `dir: 'data'`): same files, same contents.

#### Overlays
- **Command palette:** built on gpuix-svelte's `Portal` + `dismiss_layer` (the pieces
  gpuix-svelte-ui's Dialog is made of), because that Dialog's light panel with a title row and close
  button cannot take the graphite look. Same items, search, arrows and Enter (Enter is the input's
  `onsubmit`). Panel at a fixed 144 px from the top (18 % of 800; `vh` is not supported), scrim
  `#0000001a` without the Tracker's backdrop blur, no shadow. Items that open another overlay run
  right after the palette closes (no "after close animation" wait: there is no animation).
- **New issue:** gpuix-svelte-ui Dialog, size lg (640 px, vertically centred; the Tracker's is
  560 px at 14 % from the top), scrim `#00000099` (the Tracker's is 10 % black with blur), its own
  title row and close button. The four Selects are 180 px minimum, so they wrap to two lines, and
  show no glyphs. Label checkboxes as above. Cmd+Enter is an `on_window_key` handler while the dialog
  is open (the Tracker listens on DialogContent). The title field is focused by `autofocus`.
- **Shortcuts:** gpuix-svelte-ui Dialog (md, 440 px; the Tracker's is 480 px), same table, group
  headings upper-cased in the text.
- **Toasts:** app code as in the Tracker (max 3, 5 s, Undo/Open), in a Portal. Enter animation is
  a 160 ms opacity fade through GPUI `motion`; the Tracker's 8 px slide-in and `shadow-sm` are not
  available.
- No dialog open/close animations (tw-animate's fade/zoom in the Tracker).

#### Missing CSS, everywhere
`box-shadow`, `transform`/`translate`, `transition`, `z-index`, `letter-spacing`
(`tracking-*`), `text-transform`, `calc()`, `vh`/`%` for positions, `margin: auto`,
descendant/attribute selectors, `display: none`, `:last-child`, `color-mix()` and `oklch()`.
Colours were converted from the Tracker's oklch tokens to sRGB hex once (`scripts/colors.ts`), and
the avatar's `color-mix(in oklch, …)` tints were worked out the same way; alpha variants
(`/70`, `/60`, `/50`, `/[0.04]`) are 8-digit hex.

### Desktop APIs

| API | Tracker | gpuix |
|---|---|---|
| fs in the data dir | `sepal/fs` | Node/Bun `fs/promises` |
| Save dialog (Export CSV) | yes | **not supported** |
| Clipboard (Copy link) | yes | **not supported** |
| Window title | `setTitle` | `set_window_title` |
