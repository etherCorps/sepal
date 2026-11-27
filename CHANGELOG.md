# Changelog

## Unreleased

### Breaking: an app is a web build, loaded from index.html

A Sepal build is what a web build is: `dist/index.html`, the ES module
chunks it loads, its stylesheets, fonts and static files, and `app.json`
for the host. The host parses the page, runs its scripts as a browser
does, and loads modules, stylesheets and fonts from the app folder only.
There is no `app.js` any more.

A SvelteKit app uses SvelteKit's own plugin, given its options through
`withSepal()`, with Sepal's adapter, `sepal/adapter`, and `sepal()` after
it. SvelteKit's output is written as SvelteKit writes it, with its
default split build: a route's code is read when it is first opened.
`sepalKit()` is gone; for one release it throws, showing the new setup.

To move a SvelteKit app:

| Before | Now |
|---|---|
| `plugins: [sepalKit()]` from `sepal/sveltekit` | `plugins: [sveltekit(withSepal({ adapter: adapter() })), sepal()]`, with `sveltekit` from `@sveltejs/kit/vite`, `adapter` from `sepal/adapter` and `sepal`, `withSepal` from `sepal/vite` |
| `sepalKit({ kit: { … } })` | the same options in `withSepal({ … })` |
| `sepalKit({ out })` | `adapter({ out })` |
| options set by hand: nothing | `compilerOptions.experimental.customRenderer: 'sepal/renderer'`, `version.pollInterval: 0` and `sepalPreprocess()` from `sepal/vite`, last in `preprocess`; `sepal()` stops the build naming `withSepal()` when one is missing |
| `dist/app.js`, `dist/app.css` | `dist/index.html` and `dist/_app/` |

To move an app without SvelteKit:

| Before | Now |
|---|---|
| the module in `entry` (default `src/main.ts`) | `index.html` next to `vite.config.ts`, with `<script type="module" src="/src/main.ts"></script>`; `sepal()` says so when it is missing |
| `entry` in `sepal.config.ts` | gone |
| `dist/app.js`, `dist/app.css` | `dist/index.html`, `dist/assets/` |

Also:

- **The prebuilt hosts are a quarter smaller**, built with fat LTO in one
  codegen unit (the new `dist` profile) and as fast in the headless bench:
  darwin-arm64 18.5 MB, darwin-x64 20.4 MB, linux-arm64-gnu 22.2 MB,
  linux-x64-gnu 25.4 MB. Each must stay under 30,000,000 bytes; the release
  stops above that.
- **`calyx app.js app.css` is gone.** `calyx dist` runs an app from its
  folder, and `calyx --bundle` packs one that has an `index.html`. It
  replaces a bundle Sepal made, from this release or an earlier one (dev
  keeps one in `node_modules/.sepal`, for the app's name and icon in the
  Dock), and still never one it did not make.
- **Relative `fetch` URLs resolve against `http://app.localhost/`**, the
  page's address, and the host answers that origin from the app folder:
  `GET` and `HEAD`, a content type by extension, a 404 for a missing file
  or a path that leads out of the folder. No `permissions.http` is needed
  for it, and it never reaches the network. `fetch('not a url')` is now a
  404 instead of an error.
- **A minimum window size:** `window.minWidth` and `window.minHeight` in
  `sepal.config.ts` say how small the window can be made, for a layout
  that cannot work below some size. The window never opens smaller. The
  Tracker sets 720 x 520.
- **The title needs no permission.** `document.title` and
  `<svelte:head><title>` set the window's title without
  `permissions.window`, as a page's title on the web; so does
  `setTitle()` from `sepal/window`. The other window calls still need it.
- **Nothing loads from the network but `fetch`.** A script, stylesheet,
  `modulepreload` or font from `http(s)://` stops the build, with what to
  do instead (Fontsource for fonts); the host refuses them as well. What
  the host leaves out of a page (`on*` attributes, iframes, import maps,
  `document.write`) is warned about at build time.
- **`sepalKit()` throws**, with the new setup in its message.
- **A `<link rel="stylesheet">` the app adds gets a `load` or `error`
  event**, as Vite expects for a chunk's styles. `document.currentScript`
  is the running classic script's element.
- **Sepal's runtime goes first in the modules `index.html` loads**, whatever
  order the app imports in, so that its fixes for QuickJS are in place
  before Svelte's runtime keeps its own copy of what they fix. (`bind:`
  through `{...rest}` props broke when Svelte was imported first.)
- **Selectors that end in a tag do not walk the page** in the DOM layer,
  and URL parses are cached: SvelteKit's split build looks for
  `meta[property=csp-nonce]` several times a navigation.
- **A list scrolled down no longer takes clicks meant for what is above
  it.** Rows scrolled out of the top of a scroll box were still hit where
  they would have been, under the buttons drawn there; in the counter
  example "start timer" stopped working once the list was scrolled a little.
  A box that clips its overflow now clips clicks too (a Blitz patch).
- **A crash when an element came and went within one turn is fixed.** A
  form control, `<style>` or `<title>` that was added and garbage-collected
  before the turn ended (rare on QuickJS, seen on V8) stopped the app with
  "invalid SlotMap key used"; the host now frees such nodes after Blitz has
  finished with them.
- `sepal`'s peer range for `@sveltejs/kit` is now `>=3.4 <3.5`, the
  versions `sepal()`'s edits to SvelteKit's client were tested with, and
  the starter asks for `~3.4.0`. CI builds the starter against the newest
  SvelteKit once a day, when there is one newer than the lockfile's
  (`kit-latest`, allowed to fail); the range goes up once that passes.

### Modules load as QuickJS bytecode

After every build, calyx compiles each module of the app folder to
QuickJS bytecode, `x.js.qbc` next to `x.js` (`calyx --compile <dist>`;
the adapter and `sepal()` run it, dev included), and the host loads that
instead of parsing the module. The Tracker's page starts 19 to 20 ms
sooner, uses 23 ms less CPU to settle and 5.5 MB less RAM, and idles at
7.0 ms of CPU in 20 s instead of 11.4 (`compare/results/bytecode.md`).
First visits to a route barely move: there was little to compile.

What changes for apps:

- **`Function.prototype.toString` returns the `[native code]` form for
  app code.** The bytecode has no source text. Error stacks keep their
  file, line and column.
- **The app folder about doubles**: the `.js` files ship as well, for a
  host whose bytecode differs and for V8 hosts, which load only them.
  The Tracker's goes from 972 KB to 1,964 KB.
- **A build takes the compile longer**: 38-41 ms for the Tracker's 45
  modules. Without a calyx for the machine, or with one from before
  `--compile`, the build says so and the app loads its JavaScript.
- **Bytecode is used only when it fits.** Its header names the calyx
  version, the QuickJS-ng version, the target and the module, and the
  size of the `.js` it came from (in dev, its contents too). Anything else,
  a damaged file included, loads the `.js`, and the host says why. It is
  read only from the app folder.
- **`calyx --bundle` compiles again** inside the bundle, with the binary
  it bundles.

### Navigation in SvelteKit apps is faster

After every navigation SvelteKit walks the page for its links, to set up
preloading, and for `[autofocus]`. The DOM layer now resolves a link's
`href` once per attribute and page, builds a selector's test once instead
of matching it from scratch on every element, and makes a `URL`'s
`searchParams` only when it is asked for. SvelteKit's look at each link
went from 16.3 to 7.3 µs; in the Tracker the turn that renders the Board
is 5.5 ms shorter and Issues' 3.2 (`compare/results/nav-cost.md`).

- `searchParams` on the DOM layer's `URL` is no longer an own property of
  each URL, as in browsers and Node: `Object.keys(url)` no longer lists it.

### Screens drawn for the first time are faster

QuickJS-ng 0.16.2 put Map and Set keys that are numbers, objects or
symbols in a few of a table's buckets: node ids 1 to 10,000 fell in 64 of
8,192, and the renderer keys its node maps by id. The host now carries a
patched copy of `rquickjs-sys` (`crates/rquickjs-sys`) whose hash spreads
every key but a string over the buckets. In the Tracker the turn that
renders the Board for the first time is 21 ms shorter, Insights' 6 ms and
Issues' 5 ms (`compare/results/alloc.md`). A Map or Set of 10,000 integer
keys is 27 times faster to fill and search, one of objects 7 times.
Start-up, idle, memory and the host's size do not change.

Upstream fixed it in quickjs-ng #1931, which no release has yet; the
patched copy goes once rquickjs ships one that does.

### Files and `fetch` start while the window opens

A host call made while the app starts used to wait until the window
existed. Now only calls that need the window wait for it: window
controls, dialogs and the clipboard. Reading a file, `fetch`, opening a
link and the updater start as soon as the app asks, while the window is
being created. Their answers reach the app on its first turn in the
window, as before, so a call the app makes only once another has been
answered starts there too. The Tracker reads its fixture that way and
starts as fast as before (`compare/results/early-calls.md`).

## 0.2.0-alpha.0 (25 Nov 2026)

### Breaking: SvelteKit is the only router

Sepal's built-in router is gone, and with it `sepal/router`,
`sepal/navigation`, `sepal/state`, `sepal/ambient`, `virtual:sepal/routes`
and the preset's `router` option. Pages in `src/routes` are now always a
SvelteKit app, built with `sepalKit()`. `create-sepal` makes only the
SvelteKit starter, and its `--router` option is gone. An app with
`src/routes` and no `src/main` that still uses `sepal()` stops at build
time and says what to do.

To move an app from the built-in router:

| Before | Now |
|---|---|
| `sepal()` in `vite.config.ts` | `sepalKit()` from `sepal/sveltekit`, with `@sveltejs/kit` 3.4 or later in `devDependencies` |
| nothing | `src/routes/+layout.ts` with `export const ssr = false;`, and SvelteKit's `src/app.html` (copy the starter's) |
| `import { error, redirect } from 'sepal/router'` | `import { error, redirect } from '@sveltejs/kit'` |
| `$app/navigation` and `$app/state` from Sepal (`sepal/navigation`, `sepal/state`) | SvelteKit's own `$app/navigation` and `$app/state`; `page.url` is a real `URL` |
| `back()` and `forward()` from `$app/navigation` | `history.back()` and `history.forward()` |
| `"types": ["sepal/ambient"]` and `paths` in `tsconfig.json` | `"extends": "$app/tsconfig"`, and `svelte-kit sync` before `svelte-check` |
| `$lib/…` | `#lib/…` with the file's extension (`'#lib/format.ts'`), declared in `package.json`: `"imports": { "#lib/*": "./src/lib/*" }` |
| `dom: true` in `sepal.config.ts` | not needed: `sepalKit()` always loads the DOM layer |
| an unknown path showed the root `+error.svelte` | add `src/routes/[...path]/+page.ts` that throws `error(404, …)`; without it `goto` rejects and a link does nothing |
| `router: { entry }`, your own router over `virtual:sepal/routes` | `entry` in `sepal.config.ts` with `sepal()`; there is no generated route table |
| `router: false`, `router: { dir }` | gone: `sepal()` no longer reads `src/routes` |
| `pnpm create sepal --router …` | `pnpm create sepal` |

Apps without routes (`src/main.ts`, `sepal()`) do not change; the `$lib`
alias stays for them.

### Also in this release

- **`requestAnimationFrame` runs on the window's frames.** It was a 16 ms
  timer. Callbacks now run when the window draws its next frame, before it
  lays out and paints, all with that frame's timestamp; a window draws
  frames for them only while one is queued, and none while it is hidden or
  minimised, when they wait. A SvelteKit link, which waits for a frame and
  then a `setTimeout(0)`, follows sooner: opening an issue in the Tracker
  takes 19 ms instead of 35.
- **`performance.now()` is on the same clock as timers and frame
  timestamps**, on both engines. In QuickJS it was a clock of its own.
- **A timer that is already due runs in the next turn**, without waiting
  for the timer thread. From the sixth nested timer on, `setTimeout` and
  `setInterval` wait at least 4 ms, as in browsers: a chain of
  `setTimeout(fn, 0)` no longer runs as fast as the host can turn.
- **Server code stops a SvelteKit build.** `+page.server.*`,
  `+layout.server.*`, `+server.*`, `hooks.server.*` and `*.remote.*` files
  used to build and then never run. They need the backend thread, which
  Sepal does not have yet, and the build now says so.
- **No hourly version check.** SvelteKit 3 polls for a new version every
  hour by default: a wake-up on a timer in an idle app, and a request
  nothing answers. `sepalKit()` sets `version.pollInterval` to 0 unless the
  app sets it.
- **`app.js` runs in strict mode** under `sepalKit()`, as the ES module it
  was built from. As a sloppy script, a minified app could break on V8.
- **SvelteKit's route announcer no longer takes clicks.** It is a 1px box at
  the top left whose text overflows it, and Blitz hit-tests overflowing
  text. After the first navigation it took clicks on whatever was drawn
  under it.
- **The SvelteKit starter starts in a browser too.** The comment in its
  `src/app.html` named SvelteKit's body placeholder, so SvelteKit put its
  start script into the comment. Sepal found it anyway; a browser would not.
- **`sepalKit()` no longer uses `builder.rimraf`**, which SvelteKit 3.4
  deprecates.
- **The Tracker** (`examples/tracker`) is a SvelteKit app, and so is the
  Tauri build in `compare/`, from the same source. Its bundle is 30 KB
  smaller: `tailwind-variants/lite` leaves the class merging to `cn()`.

## 0.1.0-alpha.2 (24 Nov 2026)

- **`fetch` in SvelteKit apps.** `Request`, `Headers` and `Response` are
  globals now, next to `fetch`, which also takes a `Request`. SvelteKit's
  client checks `instanceof Request` on every fetch, so until now every
  `fetch` in a SvelteKit app failed with "Request is not defined".
- **Fonts and other `url()`s in SvelteKit apps.** The adapter merges
  SvelteKit's stylesheets into `app.css`; their relative `url()`s now point
  where the files are, so `@font-face` fonts load.
- **`sepal run` shows the app's name on macOS,** not "calyx": like dev, it
  runs an app with a `name` from a throwaway bundle in `node_modules/.sepal`,
  which is where macOS takes the Dock and app switcher name from.
- **`#lib` in the SvelteKit starter.** SvelteKit 3 replaced `$lib` with
  `#lib`, which a project declares in package.json's `imports`. The
  starter now does, and its `src/lib/index.ts` says how to import it.
- **`calyx --version`** prints its version, the same as the npm packages it
  ships in.
- **Releases come from CI** (`.github/workflows/release.yml`): published with
  npm trusted publishing, so with provenance, tagged `v<version>`, and with a
  GitHub Release that carries these notes and the tarballs.

## 0.1.0-alpha.1

**The host comes in one package per platform.** `@ethercorps/calyx` no longer
carries every platform's binary (50 MB to download, whatever the machine).
Each binary is its own package, and the package manager installs only the one
that fits:

- `@ethercorps/calyx-darwin-arm64`, `@ethercorps/calyx-darwin-x64`,
  `@ethercorps/calyx-linux-x64-gnu`, `@ethercorps/calyx-linux-arm64-gnu`
  (`os`, `cpu` and, on Linux, `libc` set), listed by `@ethercorps/calyx` as
  optional dependencies.
- Versions are pinned exactly from `sepal` down to the binary, and the
  `sepal` command refuses a calyx binary of another version than its own
  unless `SEPAL_HOST` is set.
- A missing platform package is named in the error, with the usual causes:
  optional dependencies turned off, or `node_modules` copied from another OS
  or CPU.
- `find_host()` from `@ethercorps/calyx` returns `{ path, version }` and
  throws when there is no binary, instead of returning `null`.

**Dist-tags.** Until there is a stable release, every prerelease moves
`latest` as well as its own tag (`alpha`), so `pnpm create sepal` and
`pnpm add sepal` get the newest alpha. 0.1.0-alpha.0 stays `latest` only
until this release.

The host binary is unchanged.

## 0.1.0-alpha.0 as Sepal (24 Nov 2026)

**Renamed from svelte-blitz to Sepal.** Published as `sepal`, `create-sepal`
and `@ethercorps/calyx`; the five `@ethercorps/svelte-blitz*` packages, which
first published 0.1.0-alpha.0, are deprecated. There are no aliases. To move
an app across:

| svelte-blitz | Sepal |
|---|---|
| `@ethercorps/svelte-blitz`, `@ethercorps/svelte-blitz-vite`, `@ethercorps/svelte-blitz-cli` | one package, `sepal` |
| `@ethercorps/svelte-blitz/fs`, `/window`, … | `sepal/fs`, `sepal/window`, … |
| `@ethercorps/svelte-blitz-vite` | `sepal/vite` |
| `@ethercorps/svelte-blitz-vite/sveltekit`, `/router`, `/navigation`, `/state`, `/ambient` | `sepal/sveltekit`, `sepal/router`, `sepal/navigation`, `sepal/state`, `sepal/ambient` |
| `svelteBlitz()`, `svelteBlitzKit()` | `sepal()`, `sepalKit()` |
| `app.config.ts` | `sepal.config.ts` |
| `virtual:svelte-blitz/routes` | `virtual:sepal/routes` |
| the `svelte-blitz` command | `sepal` |
| `pnpm create @ethercorps/svelte-blitz` | `pnpm create sepal` |
| `@ethercorps/svelte-blitz-host`, binary `svelte-blitz-host` | `@ethercorps/calyx`, binary `calyx` |
| `SVELTE_BLITZ_*` environment variables | `SEPAL_*` |
| default app id `app.svelte-blitz.<name>` | `app.sepal.<name>` |

An app that sets no `id` gets a new bundle id and data directory with the
default: macOS treats it as a different app, and its old data stays in
`app.svelte-blitz.<name>`. The starters always set an `id`.

Also new: an app with no icon of its own gets the Sepal icon.

## 0.1.0-alpha.0 (23 Nov 2026)

First release, as `@ethercorps/svelte-blitz`, `@ethercorps/svelte-blitz-vite`,
`@ethercorps/svelte-blitz-cli`, `@ethercorps/create-svelte-blitz` and
`@ethercorps/svelte-blitz-host`.
