# Handover: icons and first package release

Written 22 Nov 2026 for whoever picks this up in Claude Code. `CLAUDE.md` has the
standing rules and `README.md` the architecture; this file is the work.

**Renamed to Sepal on 24 Nov 2026.** The names below are the current ones;
what was published before the rename, and under which names, is in
`CHANGELOG.md`.

## Native layer: backend, OS APIs and per-app host builds (24 Nov 2026)

Names follow the 24 Nov rename: the project is Sepal, the crate is `sepal`, and
the prebuilt host is calyx (binary `calyx`; npm `@ethercorps/calyx`, which since
0.1.0-alpha.1 installs one of four `@ethercorps/calyx-<platform>` packages).
This is the current work; the prompt to start it is at the end of this section.

The question: Electron apps have Node, Electrobun apps have Bun, Tauri apps
have Rust. What does a Sepal app write its "backend" in, and how does it
reach the OS beyond what the host ships today (`fetch`, files, dialogs,
clipboard, open, window, updater)?

### Decisions (Shivam, 24 Nov 2026)

- **One permissions manifest for everything:** the UI, the JS backend (see
  "Later") and native plugins. Default-deny, as today. Each plugin declares the
  key that gates it, and the app grants that key in `permissions` in
  `sepal.config.ts`.
- **The Tauri way for native code.** An app that needs more than the prebuilt
  host gets a `native/` Cargo crate and builds its own host from source, with
  its plugins and commands linked in.
- **No plugins loaded at runtime over a C ABI.** This was considered and
  rejected, for four reasons:
  - a plugin crash takes the app down with it
  - the C interface would have to stay stable forever
  - every plugin carries its own copy of Rust's standard library
  - plugins that start their own threads would undo the idle-CPU numbers
- **The prebuilt host (calyx, the four platform packages) stays** for apps
  without `native/`, and those apps never need Rust or cargo. It gets a few
  more commonly used APIs (Shivam picks them, see below). Everything else
  becomes a plugin.
- **The patches stay patches.** All four patched crates remain in this repo:
  `crates/blitz-dom` and `crates/anyrender_vello_hybrid` (Blitz's), and
  `crates/vello_gpu` and `crates/wgpu-hal` (the GPU memory fixes). Apps get
  them through `[patch.crates-io]` lines that the CLI writes into
  `native/Cargo.toml`, the same four lines `crates/host/Cargo.toml` has.
- **The JS engine stays** (decided 22 Nov). With per-app builds, V8 becomes a
  per-app choice: `features = ["engine-v8"]` in `native/Cargo.toml`. The
  prebuilt host stays on QuickJS.

### Open decisions: ask Shivam, do not guess

**Which APIs join the prebuilt host.** Candidates, cheapest first:

- `process`: standard library only
- settings store: JSON in the data directory (on the "not done" list)
- OS info, environment variables, locale: `sys-locale`
- notifications (on the "not done" list)
- secrets: `keyring` 3
- file watching: `notify`

Measured on 24 Nov: secrets built as a standalone Linux library came to
1.0 MB. About 300 KB of that is its own copy of Rust's standard library and
serde_json, which the host already has, so expect less inside the host. The
rest is keyring, its D-Bus client (vendored libdbus) and crypto. If both
notifications and secrets go in, use one D-Bus stack on Linux for both.

These stay out of the prebuilt host and become plugins:

- tray and menus: `tray-icon` and `muda` need GTK on Linux
- global shortcuts
- autostart
- SQLite

**A size budget for the prebuilt host.** At 0.1.0-alpha.1 calyx is 25.4 MB
on darwin-arm64 (25,437,456 bytes), 26.3 MB on darwin-x64, 29.3 MB on
linux-arm64-gnu and 32.3 MB on linux-x64-gnu (the V8 spike measured 24.0 MB
on 22 Nov).

**The JS backend thread** (see "Later"). Two questions:

- Confirm it.
- Decide whether it stops when the window closes or keeps running. Tray apps
  need it to keep running.

**Publishing the `sepal` crate.** To crates.io, or only through git? `sepal`
was still free on crates.io on 24 Nov, after the npm packages were published.
Publishing works: the crate depends on `blitz-dom =0.3.0-beta.4`, and each
app's patch lines replace that. Either way step 4 needs something to pin to:
release.yml tags every release `v<version>` from 0.1.0-alpha.2 on (see
"Automatic releases"); earlier versions have no tag.

### Why the patches need care now

Cargo applies `[patch]` only in the workspace being built. Until now that was
always `crates/host`, so the patches always applied.

With per-app builds, the top level is the app's `native/` crate. Without the
patch lines, it compiles against the stock crates and quietly loses what each
`PATCH.md` lists: the thirteen changes to `blitz-dom`, the eleven to
`anyrender_vello_hybrid` (the glyph atlas, damage tracking), and the GPU
memory fixes in `vello_gpu` (two) and `wgpu-hal` (three; without them the
first blit costs about 80 MB of driver memory). Step 1 makes that
impossible.

### Order of work

Do these in order. Each ends with something Shivam can check.

#### 1. Make an unpatched build fail

Add a marker to each of the four patched crates, `pub const SEPAL_PATCHED: u32`,
set to the number of changes its `PATCH.md` lists: 13 in `blitz-dom`, 11 in
`anyrender_vello_hybrid`, 2 in `vello_gpu`, 3 in `wgpu-hal`. Refer to all four
from the host. A build against a stock crate then fails to compile, with an
error that names the missing `[patch.crates-io]` line.

Document the marker in each `PATCH.md`, as a `sepal patch` like the others.
When a patched crate is upgraded or a change is added, the marker changes
along with the patches.

**Check:** removing the patch lines from `crates/host/Cargo.toml` fails the
build with that message.

#### 2. The host as a library

`crates/host`, already the `sepal` crate with the `calyx` binary, becomes a
library plus a thin binary. `lib.rs` already exists and `main.rs` already uses
`sepal::app::App`. What's missing is a public builder:

```rust
sepal::App::new()
    .plugin(sepal_secrets::init())
    .command("tracker.import", import_repo)
    .run();
```

- **`command(name, fn)`:** takes `fn(T) -> Result<U, String>`, where
  `T: Deserialize` and `U: Serialize`.
  - By default it runs on a worker thread, the same way `Api::call` returns
    `Work` today (`Plan::Later` in `host_work`).
  - A main-thread variant (`Plan::Now`, like `window_call`) is for anything
    that needs the window or AppKit.
- **`Plugin`:** a name, the permission key that gates it, its commands, and
  setup and teardown hooks.
  - The built-ins (`fs`, `http`, `dialog`, `open`, `window`, `clipboard`,
    `updater`) stay as they are.
  - Plugin calls are routed after the built-ins in `host_work`.
- **`Permissions` becomes extensible,** so plugin keys are read from the
  manifest too. An unknown key is an error unless a registered plugin claims
  it. This is the rule `Permissions::parse` already follows: a typo must not
  turn into "allowed" or "not allowed".
- **The prebuilt binary** is `App::new().run()` plus whichever built-ins
  Shivam picks.

**Check:** the prebuilt release binary stays the same size, within a few KB.
All host tests pass on both engines.

#### 3. Events from Rust to JS

- Add `emit(name, json)`, callable from any thread. It delivers to JS listeners
  registered with `on(name, fn)` in `sepal`.
- Wake the event loop the way call answers do today (the `answers` channel plus
  the waker), never by polling.
- Tray clicks, shortcuts and file watching all need this.

**Check:** `idle_turns_do_no_work` still passes. An event sent from another
thread, with no window activity, reaches JS with one wake-up.

#### 4. `native/` in the CLI

- **`sepal native init`** writes `native/Cargo.toml`, with the four
  `[patch.crates-io]` lines pinned to this release (a git tag or a commit; see
  "Publishing the `sepal` crate"), and `native/src/main.rs`.
- **`sepal add <plugin>`** adds four things: the crate, the npm package,
  the `.plugin()` line and the `permissions` entry.
- **When `native/Cargo.toml` exists,** dev and `--bundle` build it with cargo
  and hand the binary over through `SEPAL_HOST`. `find_host` in
  `packages/host/index.js` already takes that variable first, and the CLI
  skips its version check for it, as a per-app build needs. Apps without
  `native/` don't change.
- **One shared `CARGO_TARGET_DIR` per machine,** in the OS cache directory.
  Stylo, wgpu and Vello then compile once, and later apps mostly just relink.
- **JS edits still reload without cargo.** Only changes under `native/`
  trigger a rebuild. Dev uses the dev profile, which is already tuned.
- **Missing toolchain:** print one clear message with the install steps:
  rustup, plus Xcode Command Line Tools on macOS or a C compiler on Linux.
  Windows is on hold.
- **Licences:** a per-app binary needs its own `THIRD-PARTY-LICENSES.txt`,
  because Stylo is MPL-2.0. `--bundle` runs `scripts/third-party-licenses.js`
  against `native/Cargo.toml`. Today that script always reads
  `crates/host/Cargo.toml` and writes into this machine's
  `packages/calyx-<platform>/`; both paths become arguments.

**Check:** `examples/native`, with one command plus the secrets plugin, builds
and runs through the CLI. Record two times: the cold build, and the build of a
second app with the same versions on the shared target directory.

#### 5. First plugin end to end: secrets

- **Crate `sepal-secrets`,** using `keyring` 3: Keychain on macOS,
  Credential Manager on Windows, Secret Service on Linux.
- **npm package `sepal-secrets`,** with `getSecret`, `setSecret` and
  `deleteSecret`. Both packages live in this repo.
- **Gated by `permissions.secrets`.**
- **A plugin template** whose `Cargo.toml` carries the patch lines, so plugin
  authors can run their own tests.

**Check:**

- A headless test covers the permission refusal and the argument handling.
- Set and get work on macOS in a bundled, signed app.

#### 6. Prebuilt additions

Once Shivam has picked them, add each one separately. Measure the release size
before and after each, and stay within the budget.

### Later, as separate work

- **The JS backend thread:** a second QuickJS runtime on its own thread.
  - Code in `src/backend/` is called from the UI through `$backend` stubs.
  - With the SvelteKit preset, remote functions run there, with same-origin
    `fetch` routed to that thread.
- **A GitHub Actions workflow** that apps can copy to build on each OS.
- **Merging update feeds across CI machines,** which per-OS builds now make
  necessary.

### Rules

- The CLAUDE.md invariants still hold, especially no. 7 (no frame loop, no
  polling). A plugin must never start a thread that wakes on an interval, and
  idle wake-ups stay at today's level with plugins loaded.
- Commands never run inside a JS turn. They are queued, the same way
  `host_call` is today.
- Default-deny: a plugin command whose permission isn't granted is refused with
  the same "add it to `permissions.<key>`" message the built-ins give.
- Do not bump the pinned Blitz versions in this work.

### Suggested prompt for Claude Code

```
Read CLAUDE.md, then the "Native layer" section of HANDOVER.md.
Do steps 1 to 5 in order. After each step, run the check it names
and show me the result before starting the next one. Anything under
"Open decisions" is mine: ask, do not guess. Keep the invariants
in CLAUDE.md.
```

## Goal

1. **Icons**, in both senses:
   - UI icons inside apps (inline SVG, Lucide, Iconify-style components) render
     correctly, including with Tailwind colours.
   - An app can set its window, taskbar and dock icon.
2. **Release** the project on npm as **`sepal`**, with the
   Rust host shipped as **prebuilt per-platform binaries** so users need no Rust
   toolchain.

## Decisions already made (by Shivam)

- Package name: `sepal`, unscoped (decided with the rename, 24 Nov 2026;
  `sepal`, `create-sepal` and `sepal-secrets` were free on npm). The host
  package stays in the `@ethercorps` scope, which already publishes
  `@ethercorps/sveltekit-og`.
- Host distribution: prebuilt binaries through npm optional dependencies (the
  esbuild pattern). Not `cargo install`.
- Draw target stays Blitz; no native controls; one codebase for all platforms.
- Tailwind is a first-class requirement, and so is performance: no frame loop,
  near-zero idle CPU (this was the complaint about gpuix-svelte).

## Open decisions — ask Shivam, do not guess

- ~~Licence.~~ Decided 22 Nov 2026: MIT. `LICENSE` is in the root and in each
  package. Still to do: a third-party licence file for the binary (Stylo is
  MPL-2.0).
- ~~GitHub repo.~~ Decided 22 Nov 2026, renamed 24 Nov: `etherCorps/sepal`. A local git
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

**Status (22 Nov 2026):** the rename to the scoped npm name is done (itself
renamed to Sepal on 24 Nov), the
crate is `publish = false`, and the package has `bin` and `files`. The repo
uses pnpm. `git init`, `LICENSE` and the package metadata are done.
`.github/workflows/ci.yml` is written but has never run: it needs the GitHub
repo. Expect to fix the Linux system-library list on its first run.

- `git init`, first commit of the tree as delivered, then work in small commits.
- Rename the npm package to `sepal`. Places that hard-code
  the old name:
  - `packages/renderer/package.json` (`name`)
  - `packages/renderer/src/vite.js` (`customRenderer: 'sepal/renderer'`
    must become the new specifier; this string ends up as an `import` in every
    compiled component)
  - `examples/*/package.json`, `examples/*/src/main.ts`,
    `examples/*/vite.config.ts`
  - `README.md`, `CLAUDE.md`
- The crate (now `sepal`, its binary `calyx`) is `publish = false`.
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
`sepal({ app: { name, icon, window: { title, width, height } } })` writes
`dist/app.json` and copies the icon next to it. Let the host accept a directory
(`calyx dist/`) and read the manifest; keep the two-path form
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
renders with no `SEPAL_HOST`. `strip = true` is on (25.3 → 21.2 MB);
the `screenshot` feature is in the shipped binary (+1.5 MB, 22.8 MB total).
Still open: the other three platform packages (add a `package.json` each and
list them in `optionalDependencies`), the CI matrix, `codegen-units`/
`opt-level` measurements, the package README, licence fields, and publishing.

**Package shape**

```
sepal                             renderer, Vite preset, CLI; no binary
@ethercorps/calyx-darwin-arm64    one file: the host binary
@ethercorps/calyx-darwin-x64
@ethercorps/calyx-linux-x64-gnu
@ethercorps/calyx-win32-x64-msvc
```

- Each platform package sets `os` and `cpu` (and `libc` on Linux) so npm
  installs only the matching one. The main package lists all of them under
  `optionalDependencies` at exactly its own version.
- The `bin` (`sepal`: `create`, `run`, `bundle`, `screenshot`) exists in
  `packages/renderer/src/cli.js`, with host lookup in `src/host.js`. What is
  left is the platform packages it looks for. Original brief:
  Add a `bin` (`sepal`) to the main package: a small Node script that
  finds the platform package with `require.resolve`, honours a
  `SEPAL_HOST` override for local builds, prints a useful message when
  no binary matches the platform, and forwards arguments. Minimum commands:
  `sepal run [dist]` and `sepal screenshot <out.png> [dist]`.
  Dev reload already exists in the Vite preset (`SEPAL_HOST` + `vite build
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
  assembles the packages, checks all versions match, and publishes the
  platform packages before the main one.
- Publish with provenance. Prefer npm trusted publishing (OIDC) over a
  long-lived token; either way the npm-side setup is Shivam's to do.
- Run `pnpm test` in the matrix on every platform that can run it headless.
  Windows and macOS have never been built: expect fixes.
- Add a dry-run path (`npm publish --dry-run`, or `npm pack` and install the
  tarballs into a scratch project) and use it before any real publish.

**Check:** in an empty directory on a machine with no Rust installed,
`npm create vite` → add `sepal` from the packed tarballs →
`npx sepal run` opens the app. Then, and only with Shivam's explicit
go-ahead, tag and publish the alpha.

**CI (23 Nov 2026).** `.github/workflows/ci.yml` runs on every push:
`pnpm test` on Linux and macOS and the host tests on V8, all passing. What
it took to get there:

- A test that compared with Node's `URL`: Node 22 (CI) and 26 (this Mac)
  disagree in four corners, which the test now skips.
- macOS CI failed two shadcn tests that pass on this Mac. A headless document
  ran scripts on real time and animations on the test's clock; the runner is
  four times slower and the two drifted apart. They are one clock now
  (`JsEngine::set_time`), and `doc.wait()` no longer sleeps.

**Windows: on hold (Shivam, 23 Nov 2026).** The CI job is still there but
only runs when started by hand (`gh workflow run ci.yml`). Where it stands,
from run 45059364101:

- The host builds with MSVC. Passing: type checks, all example builds, the JS
  tests, and the host's `bundle`, `counter`, `icons`, `manifest`, `restyle`,
  `routing`, `shadcn`, `sveltekit` and `updater` tests.
- What was fixed to get there: paths with `\` compared against Vite's and
  SvelteKit's `/` (`sveltekit.js`, `vite.js`, `routes.js`); `/logo.svg`
  resolving to `D:\logo.svg` (`net.rs` now works on URLs); and QuickJS's
  1 MB stack limit, which is about 220 calls deep with MSVC's frames, too few
  for a Bits UI page. The limit is 4 MB now and the Windows main thread is
  linked with 8 MB (`build.rs`).
- Still failing, three things. `calls_nest_deeply…` in `js.rs`: 887 calls
  deep on Windows against 4159 on macOS, under the test's floor of 2000;
  either the floor is per platform or the limit goes up there.
  `what_may_be_opened` in `tests/desktop.rs`: the path that would be opened
  comes back canonical, which on Windows is `\\?\C:\…`; the test expects
  the path it gave, and whether the shell opens a `\\?\` path is not known.
  And `test:template`: `pnpm pack` fails when started from the test.
- Never tried: opening a window, the release build, the updater installing.
  Windows is not in the release workflow's matrix.

**Release workflow (23 Nov 2026).** `.github/workflows/release.yml`. Run by
hand (Actions -> Release -> Run workflow, or `gh workflow run release.yml`)
it publishes nothing: it builds the host on macOS arm64 and x64 and Linux x64
and arm64, runs `test:template` on each with the binary just built (both
starters created from the packed packages, built, and rendered by the
packaged host), collects the four binaries and the licence file into
`packages/host`, packs the three packages (`sepal`, `create-sepal`,
`@ethercorps/calyx`) and attaches the tarballs to the run
as the `packages` artifact. That has been run once and passed. Pushing a
`v<version>` tag does the same and then publishes those tarballs with
`npm publish --provenance`; that half has never run. It needs an `NPM_TOKEN`
secret in the repository, and the tag has to equal the packages' version. A
prerelease version publishes under its own dist-tag (`0.1.0-alpha.0` as
`alpha`).

**First release: 0.1.0-alpha.0 (23 Nov 2026).** All five packages of the
layout at the time are on npm, under the names they had before the rename
(`CHANGELOG.md`). The tarballs were built by the release workflow run by hand
(run 45102868820, commit `9937a1a`), downloaded, checked, and published by
Shivam from his terminal with `pnpm publish <tarball> --access public --tag
alpha`: the account asks for a second factor, which only an interactive
terminal can answer. The registry's checksums match the tarballs. Being the
first version of each package it is `latest` as well as `alpha`. Checked from
the registry afterwards on macOS arm64: both starters created with the
published create package's `@alpha`, installed, type-checked, built and
rendered by the host that came with them.

No git tag was pushed. A `v*` tag makes the workflow publish, which needs an
`NPM_TOKEN` secret that does not exist, and that half of the workflow has
still never run. For the next release either set the secret up and tag, or
repeat what was done here: bump the three versions, push, `gh workflow run
release.yml`, download the `packages` artifact, publish `@ethercorps/calyx`
first, then `sepal`, then `create-sepal`.

**Sepal: 0.1.0-alpha.0 (24 Nov 2026).** Published that way under the new
names (release run 45367543845, commit `1654633`), and the five old packages
deprecated with "Renamed to sepal: github.com/etherCorps/sepal". Checked from
the registry: `pnpm create sepal@alpha`, installed, built, and rendered by the
calyx that came with it. Two things that cost time: an expired `npm login`
makes publishing fail with `404 Not Found`, not 401 (`npm whoami` tells), and
calyx's 50 MB tarball was not downloadable for about five minutes after its
metadata was.

The registry took about four minutes to show the new packages; a 404 right
after publishing a new name means nothing.

**Releases from 0.1.0-alpha.1 (24 Nov 2026).** The host is split by platform
(see "Package layout"), so a run packs seven packages, and publishing is
`node scripts/publish.js packs`, in CI or by hand:

- Order: the four `@ethercorps/calyx-<platform>` packages, then
  `@ethercorps/calyx`, `sepal`, `create-sepal`. Before each stage it waits
  until the previous stage's tarballs download from the registry, for up to
  20 minutes.
- A package already at the version is skipped (its dist-tags are still set),
  so a run that failed halfway is simply repeated.
- `--dry-run` prints what it would run; the release workflow does that on
  every run started by hand.

**The dist-tag rule.** A stable release is `latest`. A prerelease goes out
under its own tag (`0.1.0-alpha.1` as `alpha`) and, as long as `sepal` has
no stable release, also moves `latest`, on every package: otherwise
`pnpm create sepal` and `pnpm add sepal` keep getting the first alpha. Once
a stable release exists, prereleases leave `latest` alone.
`scripts/publish.js` applies it and says so at the top; `release.yml` repeats
it.

**Automatic releases (from 0.1.0-alpha.2).** CI publishes, nobody else. A
commit on main whose version is not on npm yet is released by
`.github/workflows/release.yml`; every other push stops at its first job.
To release:

```sh
pnpm release prerelease      # or patch, minor, or an exact version
git push origin main
```

`pnpm release` (`scripts/release.js`) sets every package, the host crate and
its Cargo.lock entry to the new version, files CHANGELOG.md's "Unreleased"
section under it with today's date, and commits "Release <version>"; it does
not push. The workflow then checks the CHANGELOG has a section for the
version, runs what ci.yml runs, builds and checks the host on all four
platforms (`calyx --version` must say the version), packs, publishes with
`scripts/publish.js`, tags `v<version>` on that commit, and makes a GitHub
Release from the CHANGELOG section with the tarballs attached (a prerelease
for prerelease versions). Re-running a failed release finishes it: what is
already on npm, tagged or released is skipped. Run by hand, on any branch,
it is a dry run.

Publishing is npm trusted publishing: no token anywhere. Shivam's one-time
setup is a GitHub environment named `npm` in the repository, and each
package trusting release.yml from it:

```sh
for p in sepal create-sepal @ethercorps/calyx \
  @ethercorps/calyx-darwin-arm64 @ethercorps/calyx-darwin-x64 \
  @ethercorps/calyx-linux-x64-gnu @ethercorps/calyx-linux-arm64-gnu; do
  npm trust github "$p" --file release.yml --repo etherCorps/sepal --env npm --allow-publish
done
```

Then, on npmjs.com, each package's trusted publisher needs "Allow npm
dist-tag" turned on. `npm trust` has no flag for it (npm 12.2.3), and without
it npm publishes but refuses to move `latest`: "This trusted publisher cannot
manage dist-tags". 0.1.0-alpha.2's first run stopped there, after one
package.

npm only trusts a package that exists. A brand-new package (a Windows calyx,
`sepal-secrets`) is published once by hand, then given the same `npm trust`
line, before CI can publish it. The publish job updates npm to 11.21 or
newer first: older npm can publish with OIDC but not set dist-tags, and the
dist-tag rule above moves `latest`.

Do not fetch a tarball's URL before it is published: the registry's CDN
caches the 404 for five minutes (`max-age=300`), and installs at that edge
fail until it expires. On 24 Nov a check made a second before `sepal`
0.1.0-alpha.1's tarball existed did exactly that. Add a query string
(`?probe=…`) to anything that probes; `scripts/publish.js` does. And do not
trust a package's listing right after a publish either: during 0.1.0-alpha.2
the CDN edge near the runner served @ethercorps/calyx-darwin-x64's listing
from before its publish for over 15 minutes, while the tarball itself was
there. `scripts/publish.js` asks for each tarball directly, and treats npm's
"cannot publish over the previously published version" as published.

## What only Shivam can do

- Choose the licence, the repo location and the first version.
- Create the GitHub repo and give Claude Code access.
- npm: confirm publish rights on `@ethercorps`, set up trusted publishing or an
  automation token.
- Hands-on checks that need eyes: the window, idle CPU, the Dock icon.
- Say "publish".

## Package layout (decided by Shivam, 22 Nov 2026; renamed 24 Nov 2026)

This replaces the "Package shape" in task 4 below and the single
`packages/renderer` the older notes refer to. With the rename to Sepal the
runtime, the Vite preset and the CLI became one package.

| Package | Directory |
|---|---|
| `sepal`: the runtime; the Vite preset and router (`sepal/vite`, `sepal/sveltekit`, `sepal/router`, with a `router` option for custom routers); the `sepal` command | `packages/sepal` |
| `create-sepal`: the seeds | `packages/create` |
| `@ethercorps/calyx`: calyx, the host; finds this machine's binary | `packages/host` |
| `@ethercorps/calyx-darwin-arm64`, `-darwin-x64`, `-linux-x64-gnu`, `-linux-arm64-gnu`: one binary each | `packages/calyx-<platform>` |

One version for all of them, pinned exactly: sepal -> calyx -> the platform
package. Until 0.1.0-alpha.0 the host package held every platform's binary,
so every install downloaded all four (50 MB). From 0.1.0-alpha.1 each
platform's binary is its own package (the esbuild pattern again), listed by
calyx as optionalDependencies with `os`, `cpu` and, on Linux, `libc`, so
only the matching one installs. `find_host()` looks for it next to calyx and,
if it is missing, names the package this machine needs. Shivam's alternative,
"the way Prisma does it" (download the matching binary on first use), still
needs release hosting.

## Added outside the original brief (22 Nov 2026)

- TypeScript in the package and the examples.
- Dev reload (`vite build --watch`), run from a throwaway bundle on macOS.
- `sepal` CLI and a starter template with `static/`.
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
- `sepal.config.ts` for app settings, so `vite.config.ts` stays small
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
  - CSS transforms in reported boxes, `getComputedStyle`. (`scroll` events
    and `scrollTop` came later; see "Scrolling" in the README.)
  - The other Bits UI components (Popover, DropdownMenu, Combobox, …) were
    not tried. They use the same pieces as Select and Tooltip.

- Keyboard and text input: Tab, Enter/Space on buttons, typing, `focus()`
  from JavaScript (`FOCUS` op). Two more Blitz patches came with it (a press
  moves the focus; `:focus-visible` matches), see `crates/blitz-dom/PATCH.md`.
- Performance pass (22 Nov 2026). Fixed: a measurement no longer lays out
  when nothing changed (opening a Select went from 153 layouts to 7), ops that
  cannot show do not cause a redraw, Blitz no longer restyles a whole list
  for one attribute change (patch 7) or everything under a parent when a
  child is added (patch 9), and the renderer keeps a glyph atlas
  (`crates/anyrender_vello_hybrid`, vendored for it). Looked at and left:
  - Turns are bound by JavaScript in QuickJS. Nothing to tune on the Rust
    side; opening a Dialog took the same 8 ms in the release build and in
    the dev build (which optimises dependencies).
  - The JavaScript engine: see "V8 spike" below.
  - Memory of a drawn window was seen at about 110 MB and at about 330 MB in
    different runs of the same page. Not understood; it is on the GPU side.
  - `codegen-units` and `opt-level` (step 4 below) were not measured: with the
    time in QuickJS they will mostly change the size.

## V8 spike (22 Nov 2026)

Question: what does a JIT engine cost and buy here? Built as a second engine
behind a cargo feature (`--features engine-v8`, `crates/host/src/js/v8_engine.rs`,
the `v8` crate 152). QuickJS is still the default and nothing ships V8.

All 63 host tests pass on both engines.

| | QuickJS | V8 |
|---|---|---|
| Release binary | 24.0 MB | 62.4 MB |
| Start to first layout, counter / shadcn | 0.18 s / 0.14 s | 0.08 s / 0.11 s |
| Peak memory, headless, counter / shadcn | 63 / 50 MB | 96 / 69 MB |
| Counter: click the button | 1.0 ms | 1.0 ms |
| Counter: select a row among 1000 | 4.8 ms | 1.8 ms |
| Counter: shuffle 1000 rows | 35 ms | 10 ms |
| shadcn: open a Dialog, first time | 8.4 ms | 9.8 ms |
| shadcn: open a Dialog, after a few times | 8.3 ms | 5.5 ms |
| shadcn: open a Select, first time | 19.7 ms | 21.1 ms |
| shadcn: open a Select, after a few times | 17 ms | 12 ms |

Timings are of a whole click, headless, on a loaded machine: read them as
ratios. What they say:

- Work that is mostly JavaScript (long lists) is about 3.5 times faster.
- A component opening is 1.4 to 1.5 times faster once it has been opened a
  few times, because half of that click is layout and styling, which the
  engine does not touch.
- The first time anything runs, V8 is no faster, and slightly slower: it
  compiles each function when it is first called and its JIT only pays off
  on code that has run a few times. A code cache made at build time would
  help the first; not tried.
- With `SEPAL_V8_FLAGS=--jitless` V8 lands between the two.

Not done, and needed before V8 could ship:

- Signing. A hardened-runtime app needs the
  `com.apple.security.cs.allow-jit` entitlement for the JIT; `bundle.rs`
  signs without entitlements today. Or ship it jitless.
- The watchdog thread is woken twice for every call into JavaScript. Cheap,
  but it is a wake-up per event that QuickJS does not have.
- `cargo fetch` now downloads the `v8` crate (large) even for a QuickJS
  build, because optional dependencies are still resolved.
- JavaScriptCore on macOS (no size cost, macOS only) was not tried.

Recommendation given to Shivam: keep QuickJS as the default; offer V8 as a
build option for apps with long lists or heavy computation, where it is worth
38 MB. Not yet decided.

## SvelteKit spike (22 Nov 2026)

Question: can SvelteKit's own client router run in the host? Yes.
`examples/sveltekit` (SvelteKit 3.4, a layout, three routes, one with a
parameter and a `load`) routes by link, by `goto` and by the back button, on
both engines. `crates/host/tests/sveltekit.rs` drives it.

Each expected blocker, and what happened:

| Expected blocker | Outcome |
|---|---|
| SvelteKit's build may not accept `customRenderer` | It does, server pass included |
| Output is HTML plus module chunks | `bundleStrategy: 'single'` gives one module; `to-blitz.mjs` turns it into a script (drops the `export`, replaces `import.meta`) and adds what the HTML's start script did |
| SvelteKit mounts its root without a renderer | One build-time edit to its `client.js` |
| No `URL`, `history`, `location` in QuickJS | Written (`url.js`, and in `dom.js`), plus `TextEncoder`/`TextDecoder`, `sessionStorage`, `pageXOffset` |
| `<svelte:head>` is refused for custom renderers | Still refused: a compile error. Not solved |

Found on the way: SvelteKit listens for link clicks on `<html>`, which the
DOM layer kept outside the event path; and its startup failed without a word
(a rejected promise nobody looks at), so the build now makes it report.

Shivam's decisions (22 Nov 2026): V8 stays an opt-in build feature;
SvelteKit is productised; `<svelte:head>` is rewritten.

Done since: `sepalKit()` in `sepal/sveltekit`
(`packages/sepal/vite/sveltekit.js`). It is a SvelteKit adapter (writes
`dist/app.js`, `app.css`, `app.json` and the static files) plus the two edits
to SvelteKit's client, and it reads `sepal.config.ts` and opens and reloads the
dev window like the other preset. `head.js` rewrites `<svelte:head>` for both
presets. The host resolves `/file` against the app directory.

`create-sepal` asks which router and has a SvelteKit starter
(`--router sveltekit`); `pnpm run test:template` creates, checks, builds and
renders both.

Still to do for SvelteKit:

- Tested with SvelteKit 3.4.0 only. The two edits (`patch` in
  `sveltekit.js`) target its internal files; try every new SvelteKit release
  before raising the peer range.
- `svelte-check` in `examples/sveltekit`. (The starter has it: SvelteKit 3
  wants `"extends": "$app/tsconfig"` and `svelte-kit sync` before the check.)
- Scroll restoration and `afterNavigate` focus handling were not looked at.
- `$app/environment`'s `browser` is true, `building` false: fine. `$env`
  modules, service workers and remote functions were not tried.

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
