# Handover: icons and first package release

Written 22 Nov 2026 for whoever picks this up in Claude Code. `CLAUDE.md` has the
standing rules and `README.md` the architecture; this file is the work.

## Goal

1. **Icons**, in both senses:
   - UI icons inside apps (inline SVG, Lucide, Iconify-style components) render
     correctly, including with Tailwind colours.
   - An app can set its window, taskbar and dock icon.
2. **Release** the project on npm as **`@ethercorps/svelte-blitz`**, with the
   Rust host shipped as **prebuilt per-platform binaries** so users need no Rust
   toolchain.

## Decisions already made (by Shivam)

- Package name: `@ethercorps/svelte-blitz`. The unscoped `svelte-blitz` is taken
  on npm by an unrelated 2022 Blitz.js port. The `@ethercorps` scope already
  publishes `@ethercorps/sveltekit-og`.
- Host distribution: prebuilt binaries through npm optional dependencies (the
  esbuild pattern). Not `cargo install`.
- Draw target stays Blitz; no native controls; one codebase for all platforms.
- Tailwind is a first-class requirement, and so is performance: no frame loop,
  near-zero idle CPU (this was the complaint about gpuix-svelte).

## Open decisions — ask Shivam, do not guess

- ~~Licence.~~ Decided 22 Nov 2026: MIT. `LICENSE` is in the root and in each
  package. Still to do: a third-party licence file for the binary (Stylo is
  MPL-2.0).
- ~~GitHub repo.~~ Decided 22 Nov 2026: `etherCorps/svelte-blitz`. A local git
  repository exists; the GitHub repo has to be created by Shivam and nothing
  has been pushed.
- Package manager: pnpm (decided 22 Nov 2026).
- **First version and dist-tag.** Suggested: `0.1.0-alpha.0` under the `alpha`
  tag, because the window has not been battle-tested.
- **Scope of "app icon".** Runtime window/dock icon only, or also a `bundle`
  command that produces a `.app` / `.exe` / `.desktop` with the icon baked in?
  This brief treats bundling as a stretch goal (task 3c).

## Order of work

Do these in order. Each ends with something Shivam can check.

### 0. Make the window real (blocker for everything else)

`crates/host/src/window.rs` compiles but has never opened a window: it was
written in a sandbox with no display. Shivam is on macOS.

- Run `pnpm install && pnpm test`, then `pnpm start`.
- Fix whatever breaks. Likely suspects: renderer initialisation
  (`VelloHybridWindowRenderer`), HiDPI scale, the first paint before fonts
  resolve, `Poll` events not arriving after input.
- Then check, by hand, in the counter example:
  - clicking the counter, the timer button and a row all work;
  - the list scrolls with the wheel or trackpad;
  - hover styles on the buttons appear;
  - **idle CPU is ~0%** in Activity Monitor with the timer stopped, and stays
    low with it running (one wake-up per second);
  - resizing the window relayouts.
- Record the results in `README.md` under "What was verified".

If the GPU renderer is the problem, `anyrender_vello_cpu` also has a window
renderer; getting a window up on the CPU first is a fine intermediate step.

### 1. Repo hygiene and rename

**Status (22 Nov 2026):** the rename to `@ethercorps/svelte-blitz` is done, the
crate is `publish = false`, and the package has `bin` and `files`. The repo
uses pnpm. `git init`, `LICENSE` and the package metadata are done.
`.github/workflows/ci.yml` is written but has never run: it needs the GitHub
repo. Expect to fix the Linux system-library list on its first run.

- `git init`, first commit of the tree as delivered, then work in small commits.
- Rename the npm package to `@ethercorps/svelte-blitz`. Places that hard-code
  the old name:
  - `packages/renderer/package.json` (`name`)
  - `packages/renderer/src/vite.js` (`customRenderer: 'svelte-blitz/renderer'`
    must become the new specifier; this string ends up as an `import` in every
    compiled component)
  - `examples/*/package.json`, `examples/*/src/main.ts`,
    `examples/*/vite.config.ts`
  - `README.md`, `CLAUDE.md`
- The crate can stay `svelte-blitz-host`; mark it `publish = false`.
- Add `LICENSE` once decided, plus `repository`, `license`, `files`,
  `keywords` and `engines` to the package.
- Add a CI workflow that runs `pnpm test` on Linux for every push. It needs no
  display. Cache `~/.cargo` and `crates/host/target`; a cold build is 8+ minutes.

**Check:** `pnpm test` green locally and in CI under the new name.

### 2. UI icons

**Status (22 Nov 2026): done on macOS**, all eight probe rows correct,
`tests/icons.rs` asserts on pixels.
- 2a: cause confirmed, with one correction to the text below: the Tailwind
  colour arrives as `lab(...)`, not `oklch(...)`. Fixed in a vendored copy of
  blitz-dom (`crates/blitz-dom`, see its `PATCH.md`). Icons do follow a
  colour change after mount; no damage flag was needed.
- 2b: `src/net.rs`. Found on the way: upstream blitz-dom panics on a relative
  or malformed URL; the vendored copy no longer does.
- 2c: full SVG element list in `ops.rs`, with a test.
- Open: **upstream PRs to Blitz for the two fixes** (needs Shivam's go-ahead,
  it is an outward action), and trying CSS `url()` and `@font-face`.

`examples/icons` is a ready-made probe and `docs/icons-probe.png` is what it
renders today. Rebuild it with the commands at the top of
`examples/icons/src/App.svelte`.

What the probe shows:

| Row | Case | Today |
|---|---|---|
| 1 | inline `<svg>`, `currentColor`, parent colour is hex | correct |
| 2 | same, parent colour from a Tailwind class | stroke icon invisible, fill icon black |
| 3 | `@lucide/svelte` components, hex colour | correct, including `size` and `strokeWidth` |
| 4 | Lucide, Tailwind colour | invisible |
| 5 | `<img>` with an SVG data URI | nothing rendered |

So the renderer side already works: `svelte:element`, SVG attributes and
third-party Svelte packages all come through. The failures are two specific
gaps.

**2a. `currentColor` with Tailwind colours.** Tailwind v4's palette is
`oklch()`. Blitz renders inline SVG by serialising the `<svg>` subtree and
handing it to `usvg`; while serialising it replaces `currentColor` with the
element's computed colour as a CSS string
(`blitz-dom/src/node/serialize.rs`, `write_outer_html_in_style`, the
`current_color` binding using `clone_color().to_css_string()`). For an oklch
colour that string is `oklch(...)`, which `usvg` cannot parse, so strokes drop
out and fills fall back to black. This cause is inferred from the code and from
rows 1 vs 2; confirm it before fixing (log the serialised SVG).

Preferred fix: convert the computed colour to sRGB before serialising, in a
fork of `blitz-dom` wired in with `[patch.crates-io]`, and open an upstream PR.
Because the host ships as binaries, a git patch does not block the release.
Rejected alternative: downlevelling oklch in the CSS at build time. Lightning
CSS emits the rgb fallback *and* the oklch declaration, and Stylo will pick the
oklch one, so it does not help.

Also test that an icon's colour **updates** when the class or inherited colour
changes after mount. Blitz builds the SVG tree during layout construction; a
colour-only restyle may not rebuild it. If it does not, find the damage flag
that forces reconstruction rather than re-inserting the node from JS.

**2b. Images and data URIs.** `SvelteDocument::new` passes no `net_provider`, so
nothing that needs fetching loads: `<img>`, CSS `url()`, `@font-face` (which
also rules out icon fonts). Add a `NetProvider` that handles:
- `data:` URLs (blitz-shell has `DataUriNetProvider` behind its `data-uri`
  feature; the headless path needs one that does not depend on the shell);
- relative and `file:` URLs resolved against the directory of `app.js`, with
  `base_url` set accordingly, and refusing paths that escape that directory.

Vite library mode inlines imported assets as data URIs (verify for Vite 8), so
`import logo from './logo.svg'` should work once `data:` does.

**2c. SVG namespace.** `createElement(name)` in the renderer API carries no
namespace, so `ops.rs` decides by name from a short hard-coded list
(`SVG_ELEMENTS`). Extend it to the full SVG element set, keeping the names that
collide with HTML (`a`, `title`, `style`, `script`) as HTML, and add a test that
fails loudly when an unknown element appears inside an `<svg>`.

**Check:** a new `crates/host/tests/icons.rs` that renders `examples/icons` with
`screenshot::render_rgba` and asserts on pixels: for each of rows 1–4, a pixel on
the stroke and one in the fill match the expected colour within a tolerance, and
row 5 is not blank. Add one test that changes the colour by clicking and asserts
the icon follows. Commit the updated `docs/icons-probe.png` with all five rows
correct, and add an "Icons" section to the README (Lucide example, colour and
size via Tailwind classes).

### 3. App and window icon

**Status (22 Nov 2026):** 3a and 3b are implemented and tested headless
(`tests/manifest.rs`). Left over:
- Shivam to confirm by eye: Dock icon and window title on macOS.
- Wayland app id from the manifest name, and the Windows taskbar icon. Both are
  platform-gated code that could not be compiled on the Mac; do them when the
  Linux and Windows builds exist (task 4 matrix).
- 3c: the macOS `.app` is done (`--bundle`, `src/bundle.rs`, `tests/bundle.rs`),
  with `--sign` (tested with a real Developer ID) and `--notarize` (written,
  never run: it needs Shivam's notarytool credentials). Not done: a `.dmg`,
  and the Windows and Linux equivalents.

There is currently no place for an app to declare anything about itself: the
host takes two file paths. Icons need one, and so will the window title and
size.

**3a. App manifest.** Extend the Vite preset so that
`svelteBlitz({ app: { name, icon, window: { title, width, height } } })` writes
`dist/app.json` and copies the icon next to it. Let the host accept a directory
(`svelte-blitz-host dist/`) and read the manifest; keep the two-path form
working for tests. Use `WindowConfig::with_attributes` in `window.rs` to apply
title and size.

**3b. Runtime icon.** One source PNG (1024×1024), decoded in the host (`png` is
already an optional dependency).
- **Windows and X11:** winit 0.31 has
  `WindowAttributes::with_window_icon(Option<Icon>)` and
  `RgbaIcon::new(rgba, width, height)`. Windows additionally has a taskbar icon
  setter in `winit-win32`.
- **macOS:** winit's window icon does nothing. The Dock icon of an unbundled
  binary has to be set at runtime with `NSApplication.applicationIconImage`
  (via `objc2-app-kit`), after the event loop has created the application.
- **Wayland:** the icon comes from a `.desktop` file matched by app id. Set the
  app id from the manifest name; there is no runtime icon to set.

**3c. Stretch: `bundle`.** A command that wraps the host binary, `app.js`,
`app.css` and the manifest into a `.app` (with a generated `.icns` and
`Info.plist`), and the Windows and Linux equivalents. Do not start this without
checking with Shivam; it is a project of its own.

**Check:** on Shivam's Mac, `pnpm start` shows the example's icon in the Dock and
the manifest's title on the window. A headless test covers manifest parsing and
PNG decoding (including a malformed icon producing a clear error, not a panic).

### 4. Release

**Status (22 Nov 2026):** the package shape works end to end on macOS arm64,
from tarballs: `packages/host-darwin-arm64`, `scripts/build-host-package.js`,
`optionalDependencies` as `workspace:*` (packed as the exact version), and
`pnpm run test:template` installs both tarballs into a scratch project and
renders with no `SVELTE_BLITZ_HOST`. `strip = true` is on (25.3 → 21.2 MB);
the `screenshot` feature is in the shipped binary (+1.5 MB, 22.8 MB total).
Still open: the other three platform packages (add a `package.json` each and
list them in `optionalDependencies`), the CI matrix, `codegen-units`/
`opt-level` measurements, the package README, licence fields, and publishing.

**Package shape**

```
@ethercorps/svelte-blitz                 renderer, Vite preset, CLI; no binary
@ethercorps/svelte-blitz-darwin-arm64    one file: the host binary
@ethercorps/svelte-blitz-darwin-x64
@ethercorps/svelte-blitz-linux-x64-gnu
@ethercorps/svelte-blitz-win32-x64-msvc
```

- Each platform package sets `os` and `cpu` (and `libc` on Linux) so npm
  installs only the matching one. The main package lists all of them under
  `optionalDependencies` at exactly its own version.
- The `bin` (`svelte-blitz`: `create`, `run`, `bundle`, `screenshot`) exists in
  `packages/renderer/src/cli.js`, with host lookup in `src/host.js`. What is
  left is the platform packages it looks for. Original brief:
  Add a `bin` (`svelte-blitz`) to the main package: a small Node script that
  finds the platform package with `require.resolve`, honours a
  `SVELTE_BLITZ_HOST` override for local builds, prints a useful message when
  no binary matches the platform, and forwards arguments. Minimum commands:
  `svelte-blitz run [dist]` and `svelte-blitz screenshot <out.png> [dist]`.
  Dev reload already exists in the Vite preset (`SVELTE_BLITZ_HOST` + `vite build
  --watch`, see README); a `dev` command only has to set that variable. It is
  not required for the first release.
- Type declarations: done. `tsc` generates `packages/renderer/types/` from the
  JSDoc on `prepare`. Still to do for the release: add `types` to `files`, and
  check the packed tarball contains them.
- The package needs its own README (install, a 10-line example, the limits).

**Binary**

- The release binary is 36 MB unstripped. Add `strip = true` and
  `codegen-units = 1`, try `opt-level = "s"` against `3`, and measure size and
  the numbers printed by `tests/counter.rs` before choosing.
- Decide whether the shipped binary includes the `screenshot` feature (CPU
  rasteriser). It is valuable for users' CI; check what it costs in size.
- Build Linux on the oldest runner image available to keep the glibc floor low.
  Document the system libraries the Linux binary needs at runtime.
- macOS arm64 binaries must be at least ad-hoc signed to run; the linker
  normally does this. Verify on a clean machine by installing from a packed
  tarball, not from the build tree.

**Pipeline**

- A release workflow triggered by a `v*` tag: build matrix (macOS arm64, macOS
  x64, Linux x64, Windows x64) → upload binaries as artifacts → one job
  assembles the five packages, checks all versions match, and publishes the
  platform packages before the main one.
- Publish with provenance. Prefer npm trusted publishing (OIDC) over a
  long-lived token; either way the npm-side setup is Shivam's to do.
- Run `pnpm test` in the matrix on every platform that can run it headless.
  Windows and macOS have never been built: expect fixes.
- Add a dry-run path (`npm publish --dry-run`, or `npm pack` and install the
  tarballs into a scratch project) and use it before any real publish.

**Check:** in an empty directory on a machine with no Rust installed,
`npm create vite` → add `@ethercorps/svelte-blitz` from the packed tarballs →
`npx svelte-blitz run` opens the app. Then, and only with Shivam's explicit
go-ahead, tag and publish the alpha.

## What only Shivam can do

- Choose the licence, the repo location and the first version.
- Create the GitHub repo and give Claude Code access.
- npm: confirm publish rights on `@ethercorps`, set up trusted publishing or an
  automation token.
- Hands-on checks that need eyes: the window, idle CPU, the Dock icon.
- Say "publish".

## Package layout (decided by Shivam, 22 Nov 2026)

This replaces the "Package shape" in task 4 below and the single
`packages/renderer` the older notes refer to.

| Package | Directory |
|---|---|
| `@ethercorps/svelte-blitz` (runtime) | `packages/core` |
| `@ethercorps/svelte-blitz-vite` (preset + router, with a `router` option for custom routers) | `packages/vite` |
| `@ethercorps/svelte-blitz-cli` | `packages/cli` |
| `@ethercorps/create-svelte-blitz` | `packages/create` |
| `@ethercorps/svelte-blitz-host` (all platform binaries in one package) | `packages/host` |

One version for all of them. The host package holds every platform's binary
under `bin/<platform>/`; the per-platform optional-dependency packages are
gone. Shivam's alternative was "the way Prisma does it" (download the matching
binary); that needs release hosting, which does not exist yet, and can be
adopted later inside `packages/host` alone. A release now has to collect all
platform binaries into `packages/host/bin` before packing it.

## Added outside the original brief (22 Nov 2026)

- TypeScript in the package and the examples.
- Dev reload (`vite build --watch`), run from a throwaway bundle on macOS.
- `svelte-blitz` CLI and a starter template with `static/`.
- SvelteKit-style file routing (`src/routes`, `load`, `$app/navigation`,
  `$app/state`). See "Routing" in the README for what differs from SvelteKit.
  Not done: generated `./$types`, `invalidate`, navigation lifecycle hooks,
  rerunning only the loads whose inputs changed.
- Updater (Tauri-style: ed25519 key, `latest.json` feed, JS API). Decided by
  Shivam: own signing key, app-driven through JS, built-in HTTPS client
  (`ureq` + rustls, +2.2 MB). Not done: installing on Windows and Linux,
  download progress, merging feeds across CI machines.
- Async host calls (`__sb_host.call` / `__sb.resolve`), which `fetch` and
  other host APIs can now be built on.
- `app.config.ts` for app settings, so `vite.config.ts` stays small
  (Shivam's request).
- Desktop APIs behind a default-deny `permissions` manifest (Shivam's
  decision): `fetch`, files, dialogs, clipboard, open, window. Not done:
  notifications, menus, tray, shortcuts, multiple windows, a settings store.

- A DOM layer for web component libraries (`dom: true`, Shivam's choice
  after the shadcn probe), then layout measurement for it (also his call):
  Switch, Checkbox, Tabs, Accordion, Dialog, Select and Tooltip from
  shadcn-svelte work with mouse and keyboard. A measurement is a forced layout
  (`__sb_host.measure`, the one exception to invariant 2, which Shivam asked
  for so that the Accordion slides open), and the host also reports boxes
  after each layout. See "Measuring the page" in the README.
  Not done:
  - CSS transforms in reported boxes, `scroll` events, `getComputedStyle`.
  - The other Bits UI components (Popover, DropdownMenu, Combobox, …) were
    not tried. They use the same pieces as Select and Tooltip.

- Keyboard and text input: Tab, Enter/Space on buttons, typing, `focus()`
  from JavaScript (`FOCUS` op). Two more Blitz patches came with it (a press
  moves the focus; `:focus-visible` matches), see `crates/blitz-dom/PATCH.md`.
- SvelteKit itself was looked at again (Kit 3.0.1) once the DOM layer existed.
  Still blocked by things that are not DOM: Kit mounts its root without a
  renderer option, its build is a server pass plus module chunks, QuickJS has
  no `URL` or `history`, and Svelte refuses `<svelte:head>` for custom
  renderers. Suggested to Shivam: a time-boxed spike, shipping Kit as a second
  router if it runs. Not started; his call.

- Performance pass (22 Nov 2026). Fixed: a measurement no longer lays out
  when nothing changed (opening a Select went from 153 layouts to 7), ops that
  cannot show do not cause a redraw, Blitz no longer restyles a whole list
  for one attribute change (patch 7), and the renderer keeps a glyph atlas
  (`crates/anyrender_vello_hybrid`, vendored for it). Looked at and left:
  - Turns are bound by JavaScript in QuickJS. Nothing to tune on the Rust
    side; opening a Dialog took the same 8 ms in the release build and in
    the dev build (which optimises dependencies).
  - The JavaScript engine. Measured the counter's 1000-row shuffle in the
    mock host under Node: 2.5 ms in V8, 5.9 ms in V8 with `--jitless`, 36 ms
    here. Half of QuickJS's time is `Array.prototype.includes` on Svelte's
    dependency lists (quadratic in the list, a native loop in V8, a call per
    element in QuickJS). Our own JavaScript (renderer.js, dom.js, shims.js)
    is about 5% of a turn, so there is nothing to win by rewriting it.
    Options, for Shivam to choose: V8 through the `v8` crate (JIT; roughly
    +30 MB on a 25 MB binary, more memory, a JIT entitlement when signed) or
    JavaScriptCore on macOS (system framework, no size, macOS only). Only
    `js.rs` knows the engine.
  - Memory of a drawn window was seen at about 110 MB and at about 330 MB in
    different runs of the same page. Not understood; it is on the GPU side.
  - `codegen-units` and `opt-level` (step 4 below) were not measured: with the
    time in QuickJS they will mostly change the size.

## Known gaps outside this brief

IME, real key presses in a window (only scripted ones were run), state-preserving HMR,
`requestAnimationFrame` tied to the display, multiple
windows, the scriptc experiment. See "Next steps" in `README.md`. Do not pull
these into the icon or release work unless one turns out to block it.

## Suggested first prompt for Claude Code

> Read CLAUDE.md, README.md and HANDOVER.md. Start with task 0 in HANDOVER.md:
> run `pnpm install && pnpm test`, then `pnpm start`, and get the window working on
> this Mac. Tell me what you see and what you changed before moving on to
> task 1. Ask me the open decisions in HANDOVER.md when you reach them; do not
> publish or push tags without my say-so.
