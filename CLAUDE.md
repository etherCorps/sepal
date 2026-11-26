# Sepal

Native Svelte apps, drawn by Blitz. Svelte apps in a native window with no
webview and no Node at runtime. Svelte components are compiled with Svelte's
custom renderer API and drive Blitz (a Rust HTML/CSS engine) through a
batched op buffer. JavaScript runs in an embedded QuickJS-ng. Styling is
plain CSS; Tailwind v4 works.

Read `README.md` for the architecture and `HANDOVER.md` for the current work.

## Naming

Anything people type stays plain and starts with `sepal`: the `sepal`
package and its subpaths (`sepal/vite`, `sepal/fs`, …), the `sepal`
command, `create-sepal`, `sepal.config.ts`, `SEPAL_*` variables, `sepal()`,
`sepalPreprocess()` and `sepal/adapter`. `withSepal()` is the one exception:
it wraps SvelteKit's options, and reads as that.
The host packages are the exception, because nobody types
them: `@ethercorps/calyx`, and the `calyx` binary of the `sepal` crate.

The plant words go in docs, the README and CLI messages, only where they
read naturally. Never force them into an API.

| Word | Means |
|---|---|
| calyx | the host, prebuilt or built per app |
| bud | an app |
| stem | the JS backend thread; its folder and `$backend` stubs keep plain names |
| graft | a native plugin; in code it stays `Plugin` and `.plugin()` |
| seed | a starter template |
| bloom | the packaged app that `--bundle` produces |

Keep the standard terms: permissions, sidecar, adapter, Vite plugin. Never
use "root" as a plant word (next to permissions it reads as root access),
nor xylem or phloem (Xilem and Floem are Rust UI frameworks). Names taken
from upstream Blitz (`crates/blitz-dom`, `crates/anyrender_vello_hybrid`,
the `blitz-*` crates, "Blitz" for the engine) stay as they are.

## Brand

The logo kit is in `assets/brand/`: the logo (mark and wordmark) for light
and dark backgrounds, the mark alone, the app icon, `favicon.svg`, and PNGs
for places without SVG. The colour is `#111714`, or white on dark. Use the
files as they are: do not redraw, recolour or re-export them.
`packages/sepal/vite/icon.png` is a byte copy of `sepal-app-icon-1024.png`
(the icon for apps without one; `test/icon.test.js` checks the copy), and
the starter's and `examples/sveltekit`'s `static/logo.svg` are copies of
the mark.

## Commands

```sh
pnpm install
pnpm run build              # builds every example: dist/index.html, its chunks, app.json
pnpm run test:js            # renderer against a JS mock host (fast, no Rust)
pnpm run test:host          # end-to-end headless tests (QuickJS + blitz-dom)
pnpm run check              # tsc on the package, svelte-check on the examples
pnpm run build:host         # the shipped host (--profile dist) -> packages/calyx-<platform>/bin/calyx, and its THIRD-PARTY-LICENSES.txt
pnpm run test:template      # pack the packages, create a project from the template, build and render it
pnpm release prerelease     # ask for a release: bump everything, file the CHANGELOG, commit (see release.yml)
pnpm test                   # check + all test suites
pnpm run screenshot         # CPU-render examples/counter to screenshot.png
pnpm start                  # open examples/counter in a window (needs a display)
pnpm dev                    # same, with reload on save (debug host + vite build --watch)
pnpm dev:tracker            # the same for examples/tracker. A plain `vite build --watch` in
                            # an example runs packages/calyx-<platform>'s binary, which is stale
                            # until `pnpm run build:host`: new host features are missing
pnpm run bundle             # examples/counter/bundle/Counter.app (release host)

pnpm --filter icons build # build another example by workspace name
cargo run --manifest-path crates/host/Cargo.toml --features screenshot -- \
  --screenshot out.png examples/icons/dist 640x360
```

The first Rust build takes several minutes (Stylo, wgpu, Vello). Use the dev
profile while iterating; it is already tuned in `Cargo.toml`.

The shipped host is built with `--profile dist` (`crates/host/Cargo.toml`):
release with fat LTO in one codegen unit, a quarter smaller and as fast,
but two to three times slower to build cold. Plain `--release` keeps thin
LTO. Measure size and speed on `dist`, which is what ships and what
`compare/` runs. Every platform's host must stay under 30,000,000 bytes:
release.yml's host job checks the binary it built (`scripts/host-size.js`)
and stops the release above it. Nothing checks it on pull requests, so look
at the size when a change adds crates or features.

`tests/counter.rs` reads `examples/counter/dist`, so build the bundle before
`cargo test`. The npm scripts do this for you.

## Layout

- `packages/sepal/` — the `sepal` package. `src/` is the runtime: `renderer.js`
  (node mirror, op buffer, event bubbling), `shims.js` (console, timers),
  `index.js` (`mount`, `__sb` entry points), `protocol.js` (opcodes),
  `host-call.js` (async calls to the host), `updater.js` (the update API),
  `fetch.js` (global `fetch`), `fs.js`, `dialog.js`, `clipboard.js`,
  `shell.js`, `window.js` (desktop APIs; public names are camelCase),
  `dom.js` (the opt-in DOM layer for web component libraries). Runs in
  QuickJS: no Node code belongs here.
- `packages/sepal/vite/` — the build side. `vite.js` (`sepal/vite`:
  `sepal()`, a web build from `index.html`, and next to `sveltekit()` the
  edits to SvelteKit's client, see the `patch` table there before touching
  a SvelteKit version; `withSepal()`, SvelteKit's options for an app;
  `sepalPreprocess()`), `adapter.js` (`sepal/adapter`: SvelteKit's output
  as SvelteKit writes it, plus `app.json`), `page.js` (what a page and its
  stylesheets may load: the app folder only), `app.js` (config file,
  manifest and dev window), `head.js` and `bindings.js` (preprocessors for
  what Svelte refuses), `sveltekit.js` (`sepalKit()`, a stub that throws the
  new setup; goes a release after 0.2.0-alpha.0). All of it runs in Node.
  `icon.png` is the Sepal icon for apps without one. Pages in `src/routes`
  are always a SvelteKit app: Sepal has no router of its own.
- `packages/sepal/cli.js` — the `sepal` command. `packages/create/` —
  `create-sepal`: the starter (`template/`, a SvelteKit app) and the
  script that copies it. `pnpm run test:template` creates, checks, builds
  and renders it.
- `packages/host/` — `@ethercorps/calyx`: no binaries, only `index.js`,
  which finds this machine's (`SEPAL_HOST` first, then the platform package
  resolved from calyx's own location) and throws, naming the package, when it
  is missing. It lists the platform packages as optionalDependencies.
- `packages/calyx-<platform>/` — `@ethercorps/calyx-darwin-arm64`,
  `-darwin-x64`, `-linux-x64-gnu`, `-linux-arm64-gnu`: `bin/calyx`, `LICENSE`
  and `THIRD-PARTY-LICENSES.txt`, with `os`/`cpu`/`libc` so only the
  matching one installs. `scripts/build-host-package.js` fills this
  machine's; the release builds each on its own runner. A stale binary there
  is used by anything that does not set `SEPAL_HOST`; the root scripts set it
  or call cargo directly.
- All seven packages share one version; `scripts/check-versions.js` enforces
  it. Pins are exact all the way down (sepal -> calyx -> platform package),
  and the CLI refuses a calyx binary of another version than sepal's unless
  `SEPAL_HOST` is set (`test/cli.test.js`).
- Releases are CI's: a commit on main whose version is not on npm is
  released by `release.yml` (trusted publishing, no token), after its tests.
  `pnpm release` (`scripts/release.js`) asks for one. `scripts/publish.js`
  publishes the packed tarballs: order, waiting for tarballs to download,
  skipping what is already up, dist-tags. `check-versions.js` keeps the
  host crate at the packages' version, which `calyx --version` prints.
- `crates/host/` — the Rust host: crate `sepal`, binary `calyx`. `app.rs` (app directory, `app.json`
  manifest, icon), `bundle.rs` (macOS `.app`), `net.rs` (resource loading:
  data: URLs and the app directory only), `updater.rs` (self-update and the
  build side of it), `api/` (desktop APIs and their permission checks), `js.rs` (the engine: `js/quickjs.rs`,
  or `js/v8_engine.rs` with `--features engine-v8`), `ops.rs` (apply batches), `scroll.rs` (scroll ops and reports),
  `document.rs` (`SvelteDocument`), `timers.rs`, `window.rs`, `screenshot.rs`,
  `testing.rs`, `hud.rs` (the performance overlay).
- `examples/counter/` — the tested example. `examples/icons/` — icon probe.
  `examples/shadcn/` — the DOM layer under a real component library.
  `examples/sveltekit/` — a SvelteKit app (`sveltekit(withSepal(…))`, `sepal()`).
  `examples/bind-rest/` — `bind:` through `{...rest}` with Svelte imported
  before Sepal (`tests/bind_rest.rs`).
  `examples/router-dom-probe/` — SvelteKit's router and Bits UI together.
  `examples/tracker/` — the benchmark app, a SvelteKit app (docs/tracker-plan.md,
  `compare/`). `examples/quakes/` — a SvelteKit app made
  with `pnpm create sepal`: live USGS feeds over `fetch`,
  a d3-geo world map, shadcn-svelte. It needs the network, so no test runs
  it; `drive … real:4` does.

## Invariants — do not break these

0. **Host calls only queue.** `__sb_host.call` (used by `host_call()` in JS)
   pushes a request and returns; the host starts the work at the end of the
   turn, on a thread, and answers through `__sb.resolve` on a later turn. It
   must never do work, or touch the document, inside the call itself.
1. **Writes are batched.** Renderer methods only append integers to the op
   buffer. Nothing calls into Rust while Svelte renders. The host pulls one
   batch per turn with `__sb.take()`.
2. **Reads never cross the boundary.** `getParent`, `getFirstChild`,
   `getNextSibling` are answered from the JS mirror. Do not add a host call to
   answer a read. **One exception, added on 22 Nov 2026 when Shivam asked for the
   Accordion to slide open, which cannot be done without it:**
   `__sb_host.measure(id)`, a forced layout as in a browser. It applies the
   pending batch, lays out and returns the node's box (and its client and
   scroll sizes), because component
   libraries change the page and measure it on the next line. It is refused
   while Blitz dispatches an event (the `RefCell` around `Page` is borrowed),
   and JS then answers from the boxes the host last *reported*
   (`SvelteDocument::report_layout` -> `__sb.layout`). Do not add a second
   exception.
3. **Node ids are allocated in JS** and never reused. Fragments have id 0 and
   never exist on the host. **One exception, added on 25 Nov 2026 when apps
   became web builds loaded from `index.html`:** the nodes the host parses
   from the page are numbered by the host, `1..n` in document order, and
   handed to JavaScript once, as `__sb_page`, before any script runs
   (`page.rs`, `page.js`). `renderer.js` builds its mirror from them and
   allocates from `n + 1`. That is the only time the host tells JavaScript
   about nodes; from then on the mirror is JavaScript's own (invariant 2).
   Do not add a second exception.
4. **`protocol.js` and `ops.rs` must stay in sync.** A new opcode needs: the
   constant in both files, the arity in `test/mock-host.js`, a case in
   `apply_batch`, and a test.
5. **`SvelteDocument::settle()` is the only place the tree is mutated** after
   startup, apart from the forced layout of invariant 2, which goes through
   the same `apply_batch`. Event handlers run synchronously inside Blitz's dispatch and are
   only safe because they write to the op buffer, not the document. The
   performance overlay's nodes (`hud.rs`) are the host's own; they too
   change only inside `settle`.
6. **Read `next_timer()` last in a turn.** Effects that run during microtasks
   can start or stop timers.
7. **No frame loop, no polling.** The event loop waits in the OS. Anything that
   wakes it on a fixed interval is a regression. `idle_turns_do_no_work` and
   `timers_wake_the_event_loop` guard this. Dev reload obeys it too: the
   host blocks on stdin, it does not watch files. A frame is drawn only when
   the page changed, a CSS animation runs, or a `requestAnimationFrame`
   callback waits for one. Layout is reported at the
   end of a turn that changed the page and after a frame the shell drew
   anyway, never on a timer. Scroll positions likewise: at the end of the
   turn that moved them. **Two exceptions:** the performance overlay, while
   it is shown, refreshes once a second through a one-shot deadline that each
   refresh sets again (`hud.rs`). Hidden, it has no nodes and sets none;
   `the_overlay_wakes_the_loop_only_while_shown` guards that. And two seconds
   after the last frame presented, one wake-up lets the renderer drop the kept
   frame and the depth buffer (`window.rs`, `IDLE_RELEASE`): once per burst of
   frames, never again until something draws.
8. **`apply_batch` must never panic.** Batches come from app code. Unknown
   nodes and opcodes are logged and skipped. Blitz indexes node storage
   directly (`doc.nodes[id]`), so check `get_node(id).is_some()` first.

## Things that will bite you

- **QuickJS frees module-level values that no live function references.** V8
  keeps the whole scope alive, so Node tests will not catch this. Anything that
  must outlive the components (the `FinalizationRegistry`, the root node) is
  pinned through `keep_alive` on `globalThis.__sb`. A registry that is itself
  collected silently never fires.
- **The bundle must run with ECMAScript built-ins only.** No `window`,
  `document`, `fetch`, or Node globals. `test/mock-host.js` runs it in a bare
  `vm` context for this reason. QuickJS-ng does provide `performance`,
  `queueMicrotask`, `WeakRef`, `FinalizationRegistry`; it does not provide
  `console` or timers (see `shims.js`). Its `performance.now()` is on real
  time, so `shims.js` replaces the object: `performance.now()`, timers and
  frame timestamps all read the host's clock (`__sb_host.now`), on both
  engines. Animation code compares a frame's timestamp with
  `performance.now()`; two clocks make animations jump, and headless tests
  stop being deterministic.
- **`bind:` on elements is rewritten by the preset** (`packages/sepal/vite/bindings.js`)
  because Svelte refuses it for custom renderers: `bind:this` and `bind:value`
  become attachments, anything else compiles and warns. Attachments and
  actions get a `BlitzNode`, which has `addEventListener`/`removeEventListener`,
  `scrollTop`/`scrollLeft`, `scrollTo`/`scrollBy`/`scrollIntoView` and little
  else of the DOM.
- **Libraries that assume a browser fail at run time, not at compile time.**
  `dom: true` in `sepal.config.ts` loads `packages/sepal/src/dom.js`, a DOM on
  top of the renderer's tree, which is enough for Bits UI's Switch, Checkbox,
  Tabs, Accordion, Dialog, Select and Tooltip. It measures through a forced
  layout (invariant 2), except inside an event handler, where it gets the
  last reported box. The same call says for how long the node is still
  animating, which is all `getAnimations()` knows: enough for Bits UI to wait
  for an exit animation. `getComputedStyle()` is blank and there are no
  `animationend` events.
- **A measurement lays the page out only if something changed since the
  last layout** (`Page::laid_out_at`). Anything that can change the page must
  clear it: `Page::apply` does for ops that can show (`ApplyStats::visible`),
  `base_mut()` and `handle_ui_event` do wholesale. Forget one and scripts
  measure a stale page; clear it too eagerly and opening a Select costs 150
  layouts instead of 7.
- **Window builds and test builds have different features.** Every test, the
  `drive` example and the packaged binary use `--features screenshot`; `pnpm
  dev`, `dev:tracker` and a plain `cargo build` do not. blitz-shell takes
  blitz-paint with no features, so SVG and scrollbar painting come from the
  host's own `blitz-paint` dependency, which `window` must enable too. When it
  did not, windows drew no inline SVG at all while every test and screenshot
  passed. A window-only problem: check `cargo tree -e features -i <crate>`
  without `screenshot` before suspecting the code.
- **Two engines, one `JsEngine`.** QuickJS is the default. `--features
  engine-v8` builds the host on V8 instead; the whole test suite passes on
  both (`cargo test --features "screenshot engine-v8"`). Anything added to
  one wrapper has to be added to the other. V8 specifics that cost time to
  find: microtasks are set to explicit (or they run inside Blitz's dispatch),
  the isolate is only entered while in use (V8 insists isolates are left in
  reverse order, and a dev reload does the opposite), finalizers need the
  platform's message loop pumped, and a script can only be stopped from
  another thread.
- **Where time goes** (measured 22 Nov 2026, see README "Performance"): turns
  are bound by JavaScript in QuickJS, an interpreter, not by Blitz. Do not
  optimise Rust before `SEPAL_TRACE=1` says layouts are the cost.
- **What GPU time costs** (measured 23 Nov 2026, driver's `accumulatedGPUTime`
  per process, the number Activity Monitor shows): at the clocks an idle app
  runs at, mostly a fixed cost per command buffer (~0.15 ms), little per pass
  and almost nothing per pixel; a 64x64 and a 2400x1600 render pass cost the
  same. wgpu submits about five command buffers a frame. Fewer frames and
  fewer submissions are what lower it, not smaller draws.
- **Where memory goes in a window** (measured 23 Nov 2026 with `footprint`):
  while frames are drawn, ~120 MB of "graphics" is Apple's GPU driver, paid
  by any Metal process on its first render pass; its first blit or compute
  work would add ~80 MB more, which the `wgpu-hal` patch avoids. macOS
  reclaims it 1-2 s after the last frame, so a memory figure means nothing
  without how long ago the last frame was. What is ours: the atlas pages,
  layer textures, a depth buffer (15 MB; without it a full repaint is ~2.5 ms
  slower) and two window surfaces (~15 MB each at 1200x800 on a 2x display).
  Two seconds after the last frame, the kept frame and the depth buffer
  are let go (`release_if_idle`, on one wake-up per burst of frames).
  On the CPU side ~36 MB of live malloc (`MallocStackLogging=1` and
  `malloc_history -allBySize` on the debug build): 12 MB QuickJS's heap, the
  rest spread thin. An unbundled host also holds its Dock icon decoded (16 MB
  "CG Image"); a bundle does not.
- **`:has()` only exists because of patch 8.** Upstream drops such rules
  while parsing. If Tailwind's `has-*` or `group-has-*` stop working after a
  Blitz update, that patch was lost.
- **Detached nodes are freed only when JavaScript collects their objects**
  (the `FinalizationRegistry` sends `DROP`). Svelte's nodes form cycles, so
  that takes a cycle collection, which an idle app never triggers: leaving a
  page of 5000 rows once kept 200k nodes and ~900 MB. The host counts what
  `REMOVE` detaches and, past `GC_AFTER_DETACHED`, collects once, a second
  later (`collect_detached` in `document.rs`). The pause is real (~250 ms
  for 200k nodes in QuickJS); fewer nodes per row is the cheaper cure.
- **Timers and frame callbacks run one per call** (`__sb.timers()` and
  `__sb.frame()` return whether they ran one) and the host runs microtasks
  in between, because libraries rely on one `requestAnimationFrame`
  callback seeing what the previous one caused. A timer already due when a
  turn ends wakes the loop at once, from the UI thread
  (`TimerWaker::set_deadline`), not through the timer thread. From the
  sixth nested timer on, a timer waits at least 4 ms, as HTML says: a
  `setTimeout(0)` chain wakes the loop about 250 times a second, not as
  fast as turns go.
- **`requestAnimationFrame` runs on the host's frames.** A turn ends by
  asking `__sb.frame_pending()`; yes sets `TurnStats::frame`, and the window
  asks for a frame. `window.rs` calls `SvelteDocument::begin_frame` on
  `RedrawRequested`, before the shell lays out and paints: every callback
  gets that frame's time, and one queued inside a callback runs in the next
  frame. Timers do not run in that turn, so a `setTimeout(0)` queued in a
  frame callback runs after the frame is painted, as in a browser. Hidden or
  minimised (`WindowEvent::Occluded`, `set_visible`), no frame is asked for
  and the callbacks wait. A callback loop that changes nothing on screen is
  paced at 60 Hz by `defer_redraw`. Headless, each 20 ms step of `doc.wait`
  is a turn and then a frame.
- **The document lives in `Rc<RefCell<Page>>`** (`document.rs`). Never hold a
  borrow of it while JavaScript runs, or a measurement silently falls back to
  old boxes. `handle_ui_event` holds one on purpose: that is what stops a
  layout in the middle of Blitz's dispatch.
- **Blitz keeps styling a node with an animation in the pass that cancels
  it,** and starts animations on `display: none` elements. The forced layout
  restyles the measured node once more for that reason.
- **Never lay out a document that has no size yet.** Blitz works out
  percentage transforms once; at 0 × 0 they stay wrong (`measuring()` in
  `document.rs` guards this).
- **Two places lay out in a window:** the shell before painting, and
  `settle()` when the app measures. They must agree on the animation time;
  `window.rs` points the shell's clock at `document::animation_clock()`.
- **Scrolling is reported, not read.** Blitz scrolls boxes itself (wheel,
  scrollbar). `JsEventHandler` notes which, without entering JS, and
  `report_scrolls` sends one `scroll` event per box at the end of the turn,
  whether or not anyone listens: its detail is where the renderer keeps
  `scrollTop`. Writes are `SCROLL`/`SCROLL_INTO_VIEW` ops, applied after the
  batch once the page is laid out (the range depends on what the batch
  added). Blitz's `get_client_bounding_rect` moves a scroll box by its own
  offset; `scroll::client_rect` undoes that for boxes the app sees. The
  viewport's scroll is not reported, and smooth scrolling is not done.
  Trackpad deltas reach blitz-shell in physical pixels and it applies them
  as CSS px; `window::logical_wheel` divides by the scale factor first.
- **Event target id 0** means "outside everything the app made". Only the
  DOM layer listens to it (`host_hooks.outside`). Key presses arrive there
  when nothing has the focus.
- **Focus has two owners that must agree.** The host moves it (a press, Tab)
  and says so with focus events; JavaScript moves it with `focus()`, which
  delivers its own events at once and tells the host with the `FOCUS` op. The
  host sends no events for a `FOCUS` op, or listeners would hear them twice.
- **On macOS Backspace is not a key press to Blitz** but a command from the
  system (`AppleStandardKeybinding("deleteBackward:")`). `doc.press` in
  `testing.rs` sends both, as a real keyboard does.
- **`preventDefault()` on `pointerup` must not cancel the click** (it does
  not in a browser, it does in Blitz). `JsEventHandler` drops it.
- **`BlitzNode`'s own fields must not take DOM names.** The node id is
  `node_id` because `id` is the HTML attribute; a collision corrupts the op
  stream without an error.
- **QuickJS gets `Object.getOwnPropertyDescriptor(proxy, key)` wrong** for
  accessors. `shims.js` wraps `Proxy` to fix it; Svelte's `bind:` through
  `{...rest}` props depends on that. V8 does not have the bug, so only the
  Rust tests see it.
- **A cycle in the tree hangs Blitz.** `insert_one` throws and `apply_batch`
  refuses the op. A script that runs for 10 s is stopped (`js.rs`), so a hang
  in JS ends with an error instead of a frozen window.
- **Svelte features unavailable to custom renderers:**
  `transition:`/`animate:`/`in:`/`out:`, `<svelte:window|body|document>`
  (`<svelte:head>` too, but `packages/sepal/vite/head.js` rewrites it),
  `css: 'injected'`, hydration, event delegation. Svelte does not bubble events
  for custom renderers; `dispatch()` in `renderer.js` does.
- **A `.svelte` file without a rune compiles in legacy mode**, where
  `<Component>` tags are resolved once and never switch. Package components
  that render other components dynamically need `<svelte:options runes />`.
- **Inline SVG is drawn without the page's CSS.** `fill="var(--land)"` in an
  SVG attribute resolves to nothing, and the shape comes out black. Give SVG
  attributes literal colours (`examples/quakes/src/lib/geo.ts`, `PALETTE`).
- **SvelteKit 3 has no `$lib`.** It is `#lib`, declared in package.json's
  `imports` (the SvelteKit starter does), and imports through it need the
  file's extension: `'#lib/format.ts'`, `'#lib/components/ui/card/index.ts'`.
  Components from the shadcn CLI or a SvelteKit 2 app import `$lib/…` and
  have to be changed. `sepal()` still aliases `$lib`, for apps without routes.
- **No `URL`, `URLSearchParams`, `location` or `history` in QuickJS.** The
  DOM layer provides them (`dom: true`, and every SvelteKit app); code that
  runs without it must not reach for them.
- **The app's code runs in strict mode, and must stay that way.** It is
  ES modules, and the host loads them as modules, which are always strict.
  Only the page's inline classic scripts are sloppy, as in a browser (they
  are SvelteKit's few lines of start-up). Run as a sloppy script, the
  minified Tracker once broke on V8 only (a renderer method missing on
  Insights), and QuickJS hid it.
- **`version.pollInterval` is 0 in a SvelteKit app, and must stay so.**
  `withSepal()` sets it and throws on more; `sepal()` and the adapter stop a
  build without it.
  SvelteKit 3 polls for a new app version every hour by default: a wake-up
  on a timer, which breaks invariant 7, and a request to `app.localhost`
  that nothing answers. Updates come through `sepal/updater`.
- **SvelteKit follows a link a frame later.** Its click handler waits a
  `requestAnimationFrame`, then a `setTimeout(0)`, before it navigates: the
  next frame the host draws, and the turn after it. Headless, a link click
  needs `doc.wait(0.04)` after it settles (the frame comes in the first
  step, the timer in the second); the Tracker's and router_dom's `click()`
  do that. `goto` does not wait.
- **A box that clips its overflow clips clicks too** (blitz-dom patch 14):
  outside an `overflow: scroll`/`hidden`/`auto` box's padding box nothing
  inside it is hit, as nothing inside it is drawn there. Upstream hit
  content scrolled out of a list under the buttons above it. A test that
  clicks something out of view (a row far down a list) has to scroll it in
  first, as a user would: `SvelteDocument::scroll_into_view`, which the
  Tracker's `click()` calls when its target is not under the pointer.
  SvelteKit's route announcer (`#svelte-announcer`, a 1px `overflow:
  hidden` box whose text overflows it) took clicks at the top left before
  this; `sepal/adapter` still gives it `pointer-events: none`.
- **An unknown address needs a route in a SvelteKit app.** SvelteKit
  answers it with a page load from the server; an app has none, so `goto`
  rejects (`navigation_route_missing`) and a link changes nothing on
  screen. A catch-all `src/routes/[...path]/+page.ts` that throws a 404
  shows the error page (the Tracker has one).
- **An error page in a SvelteKit app waits for a host call.** On any page
  with status 400 or more, SvelteKit runs `updated.check()`: a `fetch` of
  `/_app/version.json`, which the host answers from the app folder on a
  thread, in real time.
  Headless, wait for an error page with `until` (`until_shown` in
  `tests/tracker/main.rs`), not `doc.wait`: under load the answer comes
  after the virtual time has passed. The same holds for anything else that
  waits on a host call.
- **Server code stops a SvelteKit build** (`server_files` in
  `adapter.js`): `+page.server.*`, `+layout.server.*`, `+server.*`,
  `hooks.server.*`, `*.remote.*`. They need the backend thread; the check
  comes out when it lands.
- **Resolve condition.** Svelte's custom-renderer build is selected with the
  `svelte-custom-renderer` export condition (set in `vite.js`). Without it
  `svelte/renderer` resolves to a stub that throws.
- **Versions are pinned on purpose.** Blitz crates are `=0.3.0-beta.4`, winit is
  a beta, and the Svelte renderer API first shipped in 5.59.0. Check the real
  signatures in `node_modules/svelte/types/index.d.ts` (module
  `svelte/renderer`) and in `~/.cargo/registry/src/*/blitz-dom-*/src` rather
  than assuming. `anyrender_vello_hybrid 0.11` and `anyrender_vello_cpu 0.18`
  are the releases that match blitz's `anyrender 0.14`.
- **`crates/anyrender_vello_hybrid` is a patched copy too** (the window's
  renderer): glyph atlas on, `render_to_rgba` for drawing without a
  window, layer textures up to the device's limit (a full-window layer on
  a 5K display is over vello_gpu's default 4096), atlas pages of 1024 instead
  of 4096 (64 MB each), and images larger than a page in textures of their own
  (vello_gpu panics on them), and damage tracking: each frame's drawing
  commands are diffed with the last frame's and only the changed region is
  repainted, over the last frame kept in a texture of its own
  (`SEPAL_DAMAGE=0` turns it off, `SEPAL_TRACE=1` prints each
  frame's damage). A frame with nothing to draw presents nothing, so nothing
  paces the next one; the host puts that redraw off to the next 60 Hz frame
  (`defer_redraw` in `window.rs`). Never sleep in the renderer for it: input
  waits behind the sleep. See its `PATCH.md`. Same rules as below.
- **`crates/vello_gpu` is a patched copy as well**: its layer textures shrink
  again once a large layer (a Dialog's backdrop) is gone, and a scene can be
  drawn at an offset (`set_root_offset`, for damage tracking). See its
  `PATCH.md`. Same rules as below.
- **`crates/wgpu-hal` is a patched copy as well**: on Apple GPUs, buffer
  clears and uploads run in render passes instead of blit encoders, because the
  first blit or compute work in a process costs ~80 MB of driver memory. A new
  use of blits (a copy kind `render_copy.rs` does not cover, a compute pass)
  brings it back; check with `footprint` while frames are drawn. See its
  `PATCH.md`. Same rules as below.
- **`crates/blitz-dom` is a patched copy**, wired in with `[patch.crates-io]`.
  It differs from crates.io 0.3.0-beta.4 in fourteen places, all marked
  `sepal patch` (as are the changes in the other patched copies) and
  described in `crates/blitz-dom/PATCH.md`. Do not
  edit anything else there, and do not run `cargo fmt` on it.
- **Tailwind colours reach Blitz as `lab()`**, not sRGB. Anything that turns a
  computed colour into a string for another parser has to convert first.
- **Two things use the network:** the updater (`updater.rs`, HTTPS only) and
  `fetch` (`api/http.rs`, only to URLs in `permissions.http`, every redirect
  hop checked). Resources (`net.rs`) never touch the network; keep it that way.
- **Everything the page loads comes from the app folder**: modules (both
  engines' loaders), stylesheets, fonts and images, through `net.rs`, which
  maps `http://app.localhost/` to the folder (canonical paths: no way out
  through `..` or a symlink). A script, stylesheet, `modulepreload` or font
  from `http(s)://` stops the build (`vite/page.js`, for `sepal()` and the
  adapter alike), and the host refuses it as well. `fetch` to the app's own
  origin is answered from the folder too, without `permissions.http`, and
  never reaches the network.
- **Desktop APIs are default-deny.** A new one needs: a permission in
  `api::Permissions`, the check in the host (never in JS), a refusal message
  built with `denied()`, and a line in the "refused without permission" test.
- **Host calls made during startup are held** until the window polls the
  document (`SvelteDocument::for_window`). Before that there is no window and
  no running event loop: window calls would be lost and rfd dialogs panic.
- **Tests must not open dialogs, write the clipboard or open links.** They
  would act on the machine of whoever runs them.
- **App settings live in `sepal.config.ts`**, loaded by the preset with
  Vite's `loadConfigFromFile`. `vite.config.ts` only lists plugins.
- **Never unpack an update before `verify()`.** `install_archive` trusts its
  input; `Updater::download` is what earns that trust.
- **Blitz CSS gaps** (per its status page): `position: sticky`, container
  queries, 3D transforms, `overflow: auto` (use `overflow: scroll`),
  `backdrop-filter` outside the Skia backend.

## Looking at a real window without eyes

- `SEPAL_TRACE=1` makes the host print UI-thread timings to stderr:
  every click, every repaint, every change the app made to the page (with the
  number of ops, and how many layouts it took and for how long), and any
  other step that took longer than 8 ms.
- `cargo run --features screenshot --example drive -- <app dir> click:SEL
  time:0.3 text:SEL shot:out.png` (in `crates/host`) drives a built app
  headless and prints each step. See the top of `examples/drive.rs`.
  `gpushot:out.png` draws with the window's GPU renderer instead of the CPU
  one (needs a GPU; the two agree to within 2/255 when the glyph atlas is
  off, and differ only on glyph edges when it is on).
- `SEPAL_GLYPH_CACHE=0` turns the glyph atlas off in a window, to
  find out whether odd-looking text is the atlas's doing.
- In `--watch` mode stdin also accepts `move <x> <y>` and `click <x> <y>`
  (logical pixels). They enter through the same function OS events do, so a
  script can drive a real window and read the trace. This does not go through
  AppKit: it cannot show a problem in how macOS delivers events.
  `key <name>` (`key Escape`) and `type <text>` press keys; those start one
  step later, at the document, because winit cannot build a key event.
- `drive`'s `time:` moves virtual time and returns at once, so a `fetch`
  over the network is still on its way. `real:SECONDS` lets real time pass
  while the host's answers come in: `drive <app> real:4 text:SEL`.
- Positions for those commands come from `drive … rect:SEL` at the window's
  size. They go stale whenever the page's content changes.
- The performance overlay (Cmd+Option+P, Ctrl+Alt+P elsewhere; `hud` on
  stdin in `--watch` mode; `SEPAL_HUD=1` to start with it shown)
  shows fps, frame time, memory, CPU, system GPU load and the app's last
  turn. With `SEPAL_TRACE=1` each refresh is also printed as a
  `[trace] hud:` line, so a script can read the numbers. Its nodes hang off
  `<html>` after `<body>`, so `body > *` never finds them; `drive … hud:on`
  shows it headless. Its own refresh is a repaint: an idle app reads 1 fps.

## Testing approach

- Every behaviour change gets a headless test in `crates/host/tests/`. Use the
  helpers in `testing.rs` (`layout`, `query`, `click`, `text`, `rect`, `attr`);
  they go through real hit testing and Blitz's event driver.
- For anything visual, render with `screenshot::render_rgba` and assert on
  pixels, then look at the PNG yourself before trusting the assertion.
- Headless time stands still, for scripts (`performance.now()`, timers,
  `requestAnimationFrame`) and animations alike. A CSS transition, a timer
  chain or a frame callback only moves when the test calls `doc.wait(seconds)`; without it you assert on the first frame of
  an animation. Do not `thread::sleep` to let a timer come due: it will not.
  Only a document made with `for_window` runs on real time
  (`timers_wake_the_event_loop`). The two clocks were separate once, and
  macOS CI, four times slower, let timers overtake the animations they
  waited for.
- `doc.rect()` and `doc.click(node)` ignore CSS transforms; hit testing does
  not. For a translated element, use `click_at`.
- Renderer-only changes: extend `packages/sepal/test/mock-host.test.js`.
  DOM layer changes: `packages/sepal/test/dom.test.js`, and `tests/shadcn.rs`.
- Preset changes: `packages/sepal/test/`. Changes to SvelteKit's setup
  (`withSepal()`, the adapter, `patch`): also
  `tests/sveltekit.rs`, `tests/router_dom.rs` and the Tracker's tests
  (`tests/tracker/`), on both engines.
- Linux CI needs no display for any of the tests.
- **CI minutes are limited.** `ci.yml` runs on pull requests (not drafts)
  and on main only, Linux tests and the V8 host tests, and skips changes
  to Markdown, `docs/` and `compare/results/`. macOS runs only in
  release.yml, so run `pnpm test` here before every push. Push when the
  work is ready for review; earlier, open the pull request as a draft.
  `kit-latest` runs daily, and only builds when SvelteKit has a newer
  release than the lockfile's; Windows runs by hand
  (`gh workflow run ci.yml -f job=windows`). Ask before starting a release
  dry run, a manual workflow or a re-run.
- CI runs Node 22, the oldest the packages support; this Mac has 26. A test
  that compares against Node's own behaviour (`url.test.js`) has to hold on
  both: Node's `URL` changed between them in a few corners. Run such a test
  with `~/.local/share/fnm/node-versions/v22*/installation/bin/node` before
  pushing.

## Verified vs. not

Verified headless on Linux x86-64: mount, Tailwind v4 styling and layout, clicks
through hit testing, bubbling, `{#if}`, keyed `{#each}`, timers, idle behaviour,
host-side freeing after GC, CPU screenshots.

Verified on macOS arm64: all of the above through `pnpm test`, and that the
window opens and idles at 0% CPU with no wake-ups.

Keyboard and text input are verified headless (Tab, Enter and Space on
buttons, typing and editing keys in an input, arrows and Escape in Bits UI
components) and by scripted keys in a macOS window. Those scripted keys start
at the document: nothing has checked how winit and AppKit deliver real key
presses.

**Never run:** Windows builds, the window on Linux, IME, clipboard shortcuts
in a text field. On macOS, painting, clicks, wheel scrolling, hover and resize
in the window have not been confirmed by eye yet.

## Conventions

- Package source: JavaScript, tabs, single quotes, `snake_case` for internals
  (matching Svelte's own source), JSDoc types. `tsc` type-checks it
  (`checkJs`, `strict`) and emits `.d.ts` into each package's `types/`
  (generated by `prepare`, git-ignored). Do not convert the source to `.ts`.
- `packages/sepal`'s `tsconfig.json` (the runtime) has no `dom` lib on
  purpose: it turns `window`/`document` use in code that runs in QuickJS
  into a type error. Node-side files (the presets, the CLI) are checked
  separately by `tsconfig.node.json`.
- Examples are TypeScript (`main.ts`, `lang="ts"`, `vite.config.ts`).
- TypeScript stays on 6.x until `svelte-check` supports 7.
- Rust: `cargo fmt`, no warnings, errors as `Result<_, String>` at the edges.
- Comments say why, not what.

## Rules for this repo

- Do not run `npm publish`, push tags, or create GitHub releases without an
  explicit go-ahead from Shivam in the current session.
- Do not bump the pinned Blitz, winit or anyrender versions as a side effect of
  another change. If a task needs a bump, do it as its own commit and re-run
  everything.
- If a Blitz bug blocks a task, prefer a small `[patch.crates-io]` fork plus an
  upstream issue or PR over working around it in the renderer.
