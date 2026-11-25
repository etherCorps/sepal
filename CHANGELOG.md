# Changelog

## Unreleased

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
