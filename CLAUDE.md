# svelte-blitz

Svelte apps in a native window with no webview and no Node at runtime. Svelte
components are compiled with Svelte's custom renderer API and drive Blitz (a
Rust HTML/CSS engine) through a batched op buffer. JavaScript runs in an
embedded QuickJS-ng. Styling is plain CSS; Tailwind v4 works.

Read `README.md` for the architecture and `HANDOVER.md` for the current work.

## Commands

```sh
pnpm install
pnpm run build              # builds examples/counter -> dist/app.js + dist/app.css
pnpm run test:js            # renderer against a JS mock host (fast, no Rust)
pnpm run test:host          # end-to-end headless tests (QuickJS + blitz-dom)
pnpm run check              # tsc on the package, svelte-check on the examples
pnpm run build:host         # release host -> packages/host-<platform>-<arch>/ (the npm binary package)
pnpm run test:template      # pack the packages, create a project from the template, build and render it
pnpm test                   # check + all test suites
pnpm run screenshot         # CPU-render examples/counter to screenshot.png
pnpm start                  # open examples/counter in a window (needs a display)
pnpm dev                    # same, with reload on save (debug host + vite build --watch)
pnpm run bundle             # examples/counter/bundle/Counter.app (release host)

pnpm --filter icons build # build another example by workspace name
cargo run --manifest-path crates/host/Cargo.toml --features screenshot -- \
  --screenshot out.png examples/icons/dist 640x360
```

The first Rust build takes several minutes (Stylo, wgpu, Vello). Use the dev
profile while iterating; it is already tuned in `Cargo.toml`.

`tests/counter.rs` reads `examples/counter/dist`, so build the bundle before
`cargo test`. The npm scripts do this for you.

## Layout

- `packages/renderer/` — the npm package. `renderer.js` (node mirror, op buffer,
  event bubbling), `shims.js` (console, timers), `index.js` (`mount`, `__sb`
  entry points), `protocol.js` (opcodes), `vite.js` (Vite preset), `cli.js` (the
  `svelte-blitz` command), `host.js` (finds the host binary), `template/`
  (the starter that `svelte-blitz create` copies).
- `packages/host-<platform>-<arch>/` — one npm package per platform holding
  the host binary, filled by `scripts/build-host-package.js`. `host.js`
  resolves it, so a stale binary there is used by anything that does not set
  `SVELTE_BLITZ_HOST`. The root scripts set it or call cargo directly.
- `crates/host/` — the Rust host. `app.rs` (app directory, `app.json`
  manifest, icon), `bundle.rs` (macOS `.app`), `js.rs` (QuickJS), `ops.rs` (apply batches),
  `document.rs` (`SvelteDocument`), `timers.rs`, `window.rs`, `screenshot.rs`,
  `testing.rs`.
- `examples/counter/` — the tested example. `examples/icons/` — icon probe.

## Invariants — do not break these

1. **Writes are batched.** Renderer methods only append integers to the op
   buffer. Nothing calls into Rust while Svelte renders. The host pulls one
   batch per turn with `__sb.take()`.
2. **Reads never cross the boundary.** `getParent`, `getFirstChild`,
   `getNextSibling` are answered from the JS mirror. Do not add a host call to
   answer a read.
3. **Node ids are allocated in JS** and never reused. Fragments have id 0 and
   never exist on the host.
4. **`protocol.js` and `ops.rs` must stay in sync.** A new opcode needs: the
   constant in both files, the arity in `test/mock-host.js`, a case in
   `apply_batch`, and a test.
5. **`SvelteDocument::settle()` is the only place the tree is mutated** after
   startup. Event handlers run synchronously inside Blitz's dispatch and are
   only safe because they write to the op buffer, not the document.
6. **Read `next_timer()` last in a turn.** Effects that run during microtasks
   can start or stop timers.
7. **No frame loop, no polling.** The event loop waits in the OS. Anything that
   wakes it on a fixed interval is a regression. `idle_turns_do_no_work` and
   `timers_wake_the_event_loop` guard this. Dev reload obeys it too: the
   host blocks on stdin, it does not watch files.
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
  `console` or timers (see `shims.js`).
- **Svelte features unavailable to custom renderers:** `bind:` on elements,
  `transition:`/`animate:`/`in:`/`out:`, `<svelte:window|body|document|head>`,
  `css: 'injected'`, hydration, event delegation. Svelte does not bubble events
  for custom renderers; `dispatch()` in `renderer.js` does.
- **Resolve condition.** Svelte's custom-renderer build is selected with the
  `svelte-custom-renderer` export condition (set in `vite.js`). Without it
  `svelte/renderer` resolves to a stub that throws.
- **Versions are pinned on purpose.** Blitz crates are `=0.3.0-beta.4`, winit is
  a beta, and the Svelte renderer API first shipped in 5.59.0. Check the real
  signatures in `node_modules/svelte/types/index.d.ts` (module
  `svelte/renderer`) and in `~/.cargo/registry/src/*/blitz-dom-*/src` rather
  than assuming. `anyrender_vello_hybrid 0.11` and `anyrender_vello_cpu 0.18`
  are the releases that match blitz's `anyrender 0.14`.
- **Blitz CSS gaps** (per its status page): `position: sticky`, container
  queries, 3D transforms, `overflow: auto` (use `overflow: scroll`),
  `backdrop-filter` outside the Skia backend.

## Testing approach

- Every behaviour change gets a headless test in `crates/host/tests/`. Use the
  helpers in `testing.rs` (`layout`, `query`, `click`, `text`, `rect`, `attr`);
  they go through real hit testing and Blitz's event driver.
- For anything visual, render with `screenshot::render_rgba` and assert on
  pixels, then look at the PNG yourself before trusting the assertion.
- Renderer-only changes: extend `packages/renderer/test/mock-host.test.js`.
- Linux CI needs no display for any of the tests.

## Verified vs. not

Verified headless on Linux x86-64: mount, Tailwind v4 styling and layout, clicks
through hit testing, bubbling, `{#if}`, keyed `{#each}`, timers, idle behaviour,
host-side freeing after GC, CPU screenshots.

Verified on macOS arm64: all of the above through `pnpm test`, and that the
window opens and idles at 0% CPU with no wake-ups.

**Never run:** Windows builds, the window on Linux, text input, IME, keyboard
events. On macOS, painting, clicks, wheel scrolling, hover and resize in the
window have not been confirmed by eye yet.

## Conventions

- Package source: JavaScript, tabs, single quotes, `snake_case` for internals
  (matching Svelte's own source), JSDoc types. `tsc` type-checks it
  (`checkJs`, `strict`) and emits `.d.ts` into `packages/renderer/types/`
  (generated by `prepare`, git-ignored). Do not convert the source to `.ts`.
- `packages/renderer/tsconfig.json` has no `dom` lib on purpose: it turns
  `window`/`document` use in the renderer into a type error.
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
