<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/sepal-logo-white.svg">
    <img src="assets/brand/sepal-logo.svg" width="280" alt="Sepal">
  </picture>
</p>

# Sepal

**Native Svelte apps, drawn by Blitz.**

A sepal is the leaf that wraps a bud and holds up the flower. Sepal wraps your
Svelte app and holds it up natively. Formerly svelte-blitz.

Svelte apps in a native window, without a webview and without Node at runtime.

Svelte components are compiled with Svelte's custom renderer API. Instead of the
browser DOM they drive [Blitz](https://github.com/DioxusLabs/blitz), a Rust
HTML/CSS engine (Stylo for CSS, Taffy for layout, Parley for text, Vello for
painting). The JavaScript runs in an embedded QuickJS-ng. Styling is ordinary
CSS, so Tailwind works as it does on the web.

Sepal is an alpha (0.1.0-alpha.1). It runs on macOS; on Linux it builds and
passes its tests headless, but its window has not been tried yet, and Windows
is on hold. Today it has:

- **Svelte 5 and Tailwind v4 as on the web**, with pages as files in
  `src/routes`, run by SvelteKit's own router as a single-page app.
- **Component libraries written for the web**: shadcn-svelte's Switch,
  Checkbox, Tabs, Accordion, Dialog, Select and Tooltip work, mouse and
  keyboard, through an opt-in DOM layer.
- **Desktop APIs behind permissions**, off until the app lists them: `fetch`,
  files, dialogs, clipboard, opening links, the window.
- **Packaging and updates**: a macOS `.app` from one command, signed with your
  Developer ID (notarising is written but has not been run yet), and
  self-update from a signed feed.
- **No frame loop**: an idle app does no work and draws nothing, and a change
  repaints only the part of the window that changed.
- **No Rust needed to use it**: `pnpm create sepal` and a prebuilt host for
  macOS (arm64, x64) and Linux (x64, arm64).

The Tracker, an issue tracker built with SvelteKit, shadcn-svelte and
Tailwind (`examples/tracker`), in a Sepal window:

![The Tracker example in a Sepal window on macOS](docs/tracker.png)

<!-- CLIP: a short clip of the Tracker goes here. -->

## Numbers

The Tracker with 1,000 issues, on an Apple M4 Pro (macOS 27, window 1200×800
at 2×). Medians of three runs, with the range in brackets; how they were
measured, and the same app on Tauri 2 and gpuix-svelte, are in
[compare.md](compare/results/compare.md).

| | |
|---|---|
| Idle CPU, 1,000 issues | 4 ms of CPU time in 20 s (4–5), 0.02% of a core |
| Idle memory, 1,000 issues | 107 MB RAM (88–107), 43 MB GPU memory (28–43) |
| Memory after wheel-scrolling the whole list, 1,000 issues | 305 MB RAM (305–306), 220 MB GPU memory (220–221) |

## Run it

Requires Node 22+, a recent stable Rust (built here with 1.97) and, on Linux, the usual windowing libraries
(`libxkbcommon`, X11 or Wayland dev packages, a Vulkan driver).

```sh
pnpm install
pnpm test              # type checks, JS renderer test, then the headless end-to-end tests
pnpm run check         # type checks only (tsc for the package, svelte-check for the examples)
pnpm run screenshot    # render examples/counter to screenshot.png on the CPU
pnpm start             # open the example in a window
pnpm dev               # the same, reloading on every save
```

## Start a project

```sh
pnpm create sepal my-app   # TypeScript, Tailwind, file-based routes, icon, manifest, static/
cd my-app
pnpm install
pnpm dev                   # opens the app, reloads on save
pnpm start                 # build once and open it
pnpm bundle                # my-app.app
pnpm check                 # svelte-check
```

The starter is a SvelteKit app: pages are files in `src/routes`, and
SvelteKit's router runs them as a single-page app. See "Routing".

The packages are on npm as `0.1.0-alpha.1`, an alpha: expect rough edges and
changes between versions. Prebuilt hosts exist for macOS (arm64, x64) and
Linux (x64, arm64, glibc 2.35 or newer); on anything else, build the host
from `crates/host` and point `SEPAL_HOST` at it.

## Names

Anything you type is plain and starts with `sepal`: the package, its imports,
the command, `sepal.config.ts`, the `SEPAL_*` variables. The parts have plant
names, used in these docs:

| Word | What it is |
|---|---|
| bud | an app |
| calyx | the host: the native program that opens the window and runs the bud, prebuilt (`@ethercorps/calyx`) or built per app |
| seed | a starter template (`pnpm create sepal`) |
| bloom | the packaged app that `sepal bundle` (`--bundle`) produces |
| stem | the JS backend thread (planned) |
| graft | a native plugin (planned); in code it stays `Plugin` and `.plugin()` |

Permissions, sidecars, adapters and Vite plugins keep their usual names.

## Packages

| Package | What it is | Directory |
|---|---|---|
| `sepal` | The runtime (`mount`, the renderer, the op protocol, the desktop APIs), the Vite presets (`sepal/vite`, and `sepal/sveltekit` for SvelteKit apps), and the `sepal` command. The runtime has no Node code. | `packages/sepal` |
| `create-sepal` | The seeds: the starter templates. | `packages/create` |
| `@ethercorps/calyx` | Calyx, the host: the lookup that finds this machine's binary. | `packages/host` |
| `@ethercorps/calyx-<platform>` | The binary for one platform; only the matching one installs. | `packages/calyx-<platform>` |

They are released together under one version (`pnpm run check` fails if they
drift). An app depends on `sepal`; calyx comes with it.

```
sepal run [dist]
sepal bundle [dist] [out-dir] [--sign <identity>] [--notarize <keychain-profile>]
sepal screenshot <out.png> [dist] [WIDTHxHEIGHT]
```

### Where calyx, the host binary, comes from

Each platform's binary is a package of its own, so nobody who installs from
npm needs Rust, and nobody downloads the other platforms' binaries.
`@ethercorps/calyx` lists them as optional dependencies; each sets `os` and
`cpu` (and `libc` on Linux), so the package manager installs only the one
that fits. The Vite preset and the CLI ask `@ethercorps/calyx` for the path;
`SEPAL_HOST` overrides it with a host you built yourself.

| Package | Binary | Download |
|---|---|---|
| `@ethercorps/calyx-darwin-arm64` | 25.4 MB | 11.3 MB |
| `@ethercorps/calyx-darwin-x64` | 26.3 MB | 12.0 MB |
| `@ethercorps/calyx-linux-arm64-gnu` | 29.3 MB | 13.1 MB |
| `@ethercorps/calyx-linux-x64-gnu` (glibc 2.35 and newer) | 32.3 MB | 13.9 MB |

Windows is not built yet. The release workflow
(`.github/workflows/release.yml`) builds each on its own runner;
`pnpm run build:host` builds the one for this machine into
`packages/calyx-<platform>/bin/calyx`.

The versions are pinned exactly, from `sepal` down to the binary, and
`sepal run` refuses a binary whose version is not `sepal`'s. If the
platform package is missing, the error names it: usually optional
dependencies were turned off, or `node_modules` was copied from another OS
or CPU.

Each platform package also carries `THIRD-PARTY-LICENSES.txt`: the licences
of the Rust crates and of QuickJS, which are compiled into the binary
(`scripts/third-party-licenses.js` collects them from the crates themselves).

## Dev: reload on save

`vite build --watch` is the dev command (`pnpm dev` in this repository builds
the host first and points `SEPAL_HOST` at it). The preset starts the
host after the first build and tells it to reload after each later one. The window stays open and keeps its size,
position and page scroll; the app itself starts over, so component state is
lost. This is a live reload, not Svelte HMR.

- A build error leaves the last good build on screen; so does a bundle that
  throws while it loads. Fix and save again.
- Closing the window ends Vite. Stopping Vite closes the window.
- Changes to `app.json` (title, size, icon) need a restart.
- On macOS the host runs out of a throwaway bundle in
  `node_modules/.sepal/`, so the Dock and the app switcher show the
  app's name and icon in dev too. This needs `app.name` in the manifest.
- There is no file watcher and no polling in the host: Vite writes one line to
  the host's stdin per rebuild. `calyx --watch dist` run by hand
  reloads when you press Enter.


## App config

An app describes itself in `sepal.config.ts`, next to `vite.config.ts`:

```ts
import { defineConfig } from 'sepal/vite';

export default defineConfig({
	app: {
		name: 'Counter',
		id: 'com.example.counter',
		// icon: static/icon.png when it exists (PNG, square, 1024×1024 recommended); the Sepal icon otherwise
		window: { title: 'Svelte on Blitz', width: 800, height: 600 }
		// version: defaults to the one in package.json
		// updater: see "Updates"
	},
	permissions: {
		// what the app may do outside its window; see "Desktop APIs"
	}
	// entry: the module the app starts from (default src/main.ts)
	// dom: true, for component libraries written for the web; see "Component libraries"
});
```

That leaves `vite.config.ts` as a list of plugins:

```ts
export default defineConfig({
	plugins: [tailwindcss(), sepal()]
});
```

The file can also be `.js` or `.mjs`, and can export a function, which gets
Vite's `{ mode, command }`. Anything passed to `sepal({ … })` directly
overrides the file, key by key. In watch mode, edits to `app` and
`permissions` are picked up on the next build; a change to `entry` needs
Vite restarted.

The icon is a static file: put it in `static/` and it is copied with the
rest. `app.icon` names another file there if it is not called `icon.png`.
An app with neither gets the Sepal icon, written into the build as
`icon.png`; the starter relies on that until you add your own.

The build writes `dist/app.json`. Point
the host at the directory and it picks them up: `calyx dist`.
Every field is optional; `calyx app.js app.css` still works and
runs without a manifest.

| Platform | Where the icon shows |
|---|---|
| macOS | Dock, set at runtime through AppKit. Leave a transparent margin around the artwork (an 824px tile on the 1024px canvas) or it looks larger than its neighbours. |
| Windows, X11 | Window icon through winit. Not run yet. |
| Wayland | Nothing at runtime: compositors take the icon from a `.desktop` file. Not done yet. |

A missing or undecodable icon stops the app with an error naming the file.

On macOS `name` also becomes the process name, which is what the menu bar
items ("Quit Counter") use. The Dock label, the app switcher and Activity
Monitor come from the app bundle a process runs from, so `sepal run` and dev
run an app with a `name` out of a throwaway bundle in `node_modules/.sepal`.
Started by hand (`calyx dist`), or without a `name`, it shows as `calyx`.

## Bundle a macOS app

The bundle is the bloom: the bud and its calyx, packed as one `.app`.

```sh
pnpm run bundle        # examples/counter/bundle/Counter.app
calyx --bundle dist [out-dir]
```

```
Counter.app/Contents/
  Info.plist           name, id, version, icon
  MacOS/Counter        the host binary that did the bundling
  Resources/           everything from dist/, plus AppIcon.icns
```

The manifest needs a `name`. `id` (default `app.sepal.<name>`) and
`version` (default `0.1.0`) should be set for anything you ship. The icon must
be square; the `.icns` is generated from it at every size up to its own.
Double-clicking the app runs the host with no arguments, and it loads the app
from `Resources`.

- Without `--sign` the bundle is ad-hoc signed, which is enough to run it on
  the Mac that built it.
- It replaces a previous bundle of the same name, and refuses to touch an
  `.app` it did not build.
- Use a release build of the host (`pnpm run bundle` does). The binary is about
  25 MB today.
- macOS only. There is no Windows or Linux packaging yet.

### Signing for other Macs

A downloaded app only opens without warnings if it is signed with a Developer
ID certificate and notarised by Apple.

```sh
# once: store an app-specific password in the keychain under a profile name
xcrun notarytool store-credentials my-profile --apple-id you@example.com --team-id TEAMID

sepal bundle dist --sign "Developer ID Application: Your Name (TEAMID)" --notarize my-profile
```

`--sign` signs with the hardened runtime and a secure timestamp; the host
needs no entitlements. `--notarize` zips the app, submits it with `notarytool`,
waits for the result and staples the ticket to the bundle. `--sign` alone has
been run here with a real Developer ID and the result launches; Gatekeeper
then reports "Unnotarized Developer ID" until it is notarised. **`--notarize`
has not been run**: it uploads to Apple and needs your credentials.

## Routing

Pages are files in `src/routes`, with SvelteKit's conventions, and SvelteKit's
own client router runs them: links, `goto`, parameters, `load`, layouts,
error pages, `$app/state`, `$app/navigation` and the back button. The app is
a SvelteKit single-page app; the starter is one, and so are
`examples/tracker`, `examples/sveltekit` and `examples/quakes`.

In `vite.config.ts`, `sepalKit()` takes the place of `sveltekit()`:

```ts
import { sepalKit } from 'sepal/sveltekit';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sepalKit()]
	// sepalKit({ kit: { … } }) passes options on to SvelteKit
});
```

and the root layout turns server rendering off, since there is no server:

```ts
// src/routes/+layout.ts
export const ssr = false;
```

Everything else is as for any Sepal app: settings in `sepal.config.ts`,
the icon and other files in `static/`, `vite build` writes `dist/`,
`vite build --watch` opens the app and reloads it on every save, and
`sepal run` and `bundle` work on the result. Needs `@sveltejs/kit` 3.4 or
later in the project, and SvelteKit's `src/app.html`; no adapter. Run
`svelte-kit sync` before `svelte-check` (the starter's `check` does), since
the project's `tsconfig.json` extends the one it writes.

```ts
// src/routes/items/[id]/+page.ts
import { error } from '@sveltejs/kit';
import { items } from '#lib/items.ts';

export function load({ params }) {
	const item = items.find((item) => item.id === params.id);
	if (!item) error(404, `No item ${params.id}`);
	return { item };
}
```

What is different from SvelteKit on the web:

- **No server.** The app is a single-page app, always. `+page.server.*`,
  `+layout.server.*`, `+server.*`, `hooks.server.*` and remote functions
  (`*.remote.*`) stop the build: they would need the app's backend thread,
  which Sepal does not have yet. Form actions and prerendering do not exist
  here. A `load` in `+page.ts` runs in the app.
- **Unknown addresses need a route.** SvelteKit answers an address no route
  takes with a page load from the server; an app has neither, so `goto`
  rejects and a link does nothing. A catch-all route that throws a 404
  shows your error page instead (`src/routes/[...path]/+page.ts` in the
  Tracker).
- **`#lib`, not `$lib`.** SvelteKit 3 has no `$lib`: declare `#lib` in
  package.json's `imports` (the starter does) and import with the file's
  extension: `'#lib/format.ts'`.
- **The address is made up.** The app starts at `http://app.localhost/`, and
  nothing is ever fetched from there. Static files are addressed from the
  root as usual (`<img src="/logo.svg">`).
- **The DOM layer is always there.** `sepalKit()` loads it, so `dom: true` is
  not needed.
- **`<svelte:head>`** is rewritten: its `<title>` sets `document.title`, which
  is the window's title when the app has `permissions.window`. Anything else
  in it is dropped with a warning, since an app has no `<head>`.
- **One bundle.** All pages are in `app.js`; there is no code splitting.
- **No version polling.** SvelteKit checks for a new version every hour by
  default; `sepalKit()` turns that off, as it would wake an idle app.
  Updates come through `sepal/updater`.
- **A link waits a frame.** SvelteKit lets the page repaint before it follows
  a link (a `requestAnimationFrame`, then a `setTimeout(0)`). In Sepal today
  that is a 16 ms timer and one more turn, about 20 ms more than a `goto`.
- **No scroll restoration**, and whatever else needs a part of the browser
  that the DOM layer does not have (see "Component libraries").

How it works, and what can break: the build edits two places in SvelteKit's
own client code as it is bundled (to load the DOM layer first, and to mount
SvelteKit's root component with this renderer), then turns SvelteKit's
output, an HTML page that imports one module, into the script the host
loads. Those edits follow SvelteKit's source. If a SvelteKit release changes
those lines, the build stops and says so. Tested with 3.4.0.

### Apps without routes

An app with one screen, or a router of its own, uses `sepal()` from
`sepal/vite` and starts from `src/main.ts`, which mounts the app
(`examples/counter`, `examples/shadcn`). `sepal()` stops the build if it
finds `src/routes` and no `src/main`, and points to `sepalKit()`.

## Component libraries: shadcn-svelte

Tested with shadcn-svelte 1.8 (its CLI, the `vega` preset, 16 components).

**The styling system works completely**: the Tailwind v4 theme, its `oklch()`
variables, variants (`tailwind-variants`), Lucide icons and the Inter font all
render as on the web.

| Works | Works with `dom: true` | Does not work yet |
|---|---|---|
| Button, Badge, Card, Alert, Skeleton, Separator, Progress, Label, Input (displaying a value) | Switch, Checkbox, Tabs, Accordion, Dialog, Select, Tooltip, with mouse and keyboard, opening and closing animated | Other Bits UI components: not tried |

The first column is markup and classes. Everything else is built on Bits UI,
which is written for a browser: it reads `window` and `document`, listens on
them, creates elements, moves focus and measures boxes.

### `dom: true`

```ts
// sepal.config.ts
export default defineConfig({ dom: true });
```

This builds the app as a browser bundle (the `browser` export condition) and
loads `sepal/dom` before anything else. That module puts a
small DOM on top of the renderer's own tree: `window`, `document`, selectors
(`querySelector`, `matches`, `closest`), attributes, `classList`, `style`,
`dataset`, tree edits, `Event` and its subclasses, `EventTarget`,
`AbortController`, listeners on `document` and `window`, focus tracking. Every
write goes through the same op buffer as Svelte's, so nothing about how the
host works changes. It is opt-in: an app that does not ask for it gets none of
it, and the rule that the bundle runs on ECMAScript built-ins alone still
holds for the layer itself.

### Measuring the page

Layout happens in the host. A measurement reaches JavaScript in two ways.

**Asked.** `getBoundingClientRect()`, `offsetWidth` and the like call
`__sb_host.measure(id)`, which is a forced layout, as in a browser: the host
applies what the app has written so far, lays the page out and returns the
node's box. Code that changes the page and measures it on the next line
depends on that. An Accordion slides open because Bits UI turns its animation
off, measures the content's full height, and turns it back on, all in one go.
This is the one place where JavaScript reads from the host.

**Told.** The first time a node is measured it is also marked with one op,
`MEASURE`. After each layout the host sends the app the window's size and the
box of every marked node that moved or changed size. `ResizeObserver` and the
`resize` event run on that, at the end of the turn, before the frame is
drawn, and again while the app keeps changing the page in answer (up to 8
times). That is how a Select's list is under its trigger in the first frame
it is visible in.

Inside an event handler the host cannot lay out, because Blitz is in the
middle of the document. A measurement made there is answered from the last
report: correct unless the handler itself changed the page first.

Not there:

- Boxes do not include CSS transforms (a dialog centred with `translate`
  reports where it would be without it).
- `getComputedStyle()` returns an empty style. Floating UI copes: it treats
  everything as unclipped and positions against the window.
- `element.getAnimations()` returns one object for everything animating on
  the element, finished when the last animation or transition is. That is what
  Bits UI waits on before it removes something, so an Accordion slides shut
  and a Dialog fades out. It is not the Web Animations API: nothing can be
  paused, sped up or read. There are no `animationend` events.
- Reopening an Accordion within 0.2 s of closing it can measure a height that
  is part-way through the animation Blitz is still running on it.
- A popup does not follow its anchor while a box scrolls under it, as far as
  is known: Floating UI finds the boxes that scroll through
  `getComputedStyle()`. Not tried since `scroll` events exist.

A page that shows a `Skeleton` never goes idle: its pulse is an animation
that does not end, and Blitz redraws while one is running.

Some things had to be fixed underneath to get this far, and they apply to
any app:

- QuickJS answers `Object.getOwnPropertyDescriptor(proxy, key)` with a data
  descriptor even when the proxy's trap returned a getter and setter. Svelte
  asks that to find the setter of a bound prop that passes through
  `{...rest}` props, so `bind:` stopped at the first wrapper component.
  `shims.js` works around it.
- Blitz hit-tested a box in a line of text (a `<button>` that is not a flex
  or grid item) as if it were shifted by its container's padding. Fixed in
  the patched `blitz-dom`.
- Capture-phase listeners now run in the capture phase.
- An event that hits nothing the app made (the page around `<body>`, or a
  `<body>` with `pointer-events: none`) used to be dropped. With the DOM
  layer it goes to `document` with `<html>` as its target, as in a browser.

### Scrolling

Works with or without `dom: true`, so a list can render only the rows in
view. Blitz scrolls a box when the wheel, trackpad or scrollbar moves it; at
the end of that turn the host tells the app with a `scroll` event on the box
(it does not bubble, as in a browser), so Svelte's own `onscroll` works.
`scrollTop` and `scrollLeft` read where the host last said the box is.
Writing them, `scrollTo()`, `scrollBy()` and `scrollIntoView()` (`block` and
`inline`: `start`, `center`, `end`, `nearest`) are ops, done at the end of the
turn after the page is laid out, so a list can grow and scroll to its new end
in one go. The host clamps to what the box can scroll and reports where it
ended up with a `scroll` event, as a browser does for a script's scroll too. A
write reads back at once, clamped to the range last reported.

With `dom: true`, `clientWidth`, `clientHeight`, `scrollWidth` and
`scrollHeight` come with the forced layout (see "Measuring the page"), or,
inside an event handler, from the last `scroll` event.

Not there: the window's own scroll (`window.scrollY`, `window.scrollTo`), for
an app whose `<body>` is taller than the window; smooth scrolling
(`behavior: 'smooth'`, `scroll-behavior`) jumps; keyboard scrolling (Page
Down, arrows), which Blitz does not do. `scrollIntoView()` moves the boxes
around the node, not the window, and `scrollTop` after it reads the old
position until the end of the turn.

### Keyboard

Works, with or without `dom: true`: Tab and Shift+Tab move the focus, Enter
and Space press the focused button, link or checkbox, typing and the editing
keys change a text input (`bind:value` follows), and `keydown`/`keyup` reach
the app with `key`, `code` and the modifier flags. A focus ring styled with
`:focus-visible` shows for keyboard focus and not after a click.

With `dom: true`, `element.focus()` and `blur()` move the real focus, and
`document.activeElement` follows whoever moved it. That is what Bits UI needs:
a Select takes arrow keys, Enter and Escape, a Dialog moves the focus into
itself, keeps Tab inside, closes on Escape and gives the focus back.

Not there: IME, `:focus-within`, and a focus ring that knows whether a
`focus()` call followed a click (it always shows). Clipboard shortcuts in a
text field exist in Blitz but have not been tried.

Out of the box none of it compiled, because every shadcn component has
`bind:this={ref}` on its root element and Svelte refuses that for custom
renderers. The preset rewrites element bindings before Svelte sees them
(`packages/sepal/vite/bindings.js`), which applies to any component library.

`examples/shadcn` is a page of the working components. `node probe.mjs` in
that directory compiles and runs each file in `src/probes` and reports
whether it loads; `cargo run --features screenshot --example drive` (in
`crates/host`) clicks through a built app and prints what happened.

## Icons and images

![The icon probe](docs/icons-probe.png)

Icons are ordinary Svelte: inline `<svg>`, or a component library. They take
their colour from `currentColor` and their size from props or classes, so
Tailwind works on them directly.

```svelte
<script lang="ts">
	import { Heart, Settings } from '@lucide/svelte';
</script>

<span class="text-rose-600"><Heart /></span>
<Settings class="size-8 text-slate-500" strokeWidth={1.5} />
<svg class="size-6 text-indigo-600" viewBox="0 0 24 24" fill="currentColor">…</svg>
```

Colours follow later changes too: switch a class on a parent and the icons
inside repaint.

Images load from two places and nowhere else:

```svelte
<script lang="ts">
	import logo from './logo.svg'; // Vite inlines it as a data: URL
</script>

<img src={logo} alt="" />
<img src="photo.png" alt="" />   <!-- a file next to app.js: put it in static/ -->
```

- `data:` URLs, which is what an `import` of an asset becomes.
- Files inside the app directory (`dist`, or `Resources` in a bundle).
  Everything in the project's `static/` directory is copied there by the
  build, as it is. A path that leaves the app directory, through `..` or a
  symlink, is refused.
- Resources never come from the network: `http(s):` URLs do not load. A URL
  that cannot be loaded is reported on stderr and the image stays empty.

CSS `url()` and `@font-face` go through the same loader but have not been
tried yet.

## Updates

A bundled app can update itself, in the way Tauri's updater works: it reads a
small feed, downloads the new version, checks its signature against a key
built into the app, replaces itself and restarts.

**1. Make a key pair, once.**

```sh
sepal signer generate ~/.keys/my-app.key
# prints the public key
```

The file is the private key. Keep it secret and backed up: updates signed
with any other key are refused by every copy of the app already installed.

**2. Tell the app where updates come from**, in `sepal.config.ts`.

```ts
export default defineConfig({
	app: {
		name: 'My App',
		version: '1.0.0',
		updater: {
			url: 'https://github.com/you/my-app/releases/latest/download/latest.json',
			pubkey: '…the public key…'
		}
	}
});
```

**3. Offer the update in the app.**

```svelte
<script lang="ts">
	import { check } from 'sepal/updater';

	async function update() {
		const found = await check();        // null when up to date
		if (!found) return;
		console.log(found.version, found.notes);
		await found.download();             // fetched and verified; nothing replaced yet
		await found.install();              // replaces the app and starts the new version
	}
</script>
```

**4. Release a new version.** Raise `version`, build, and bundle with the key:

```sh
vite build
sepal bundle --sign "Developer ID Application: …" --notarize my-profile \
  --update-key ~/.keys/my-app.key --update-notes "What changed"
```

Next to the `.app` this writes three files. Upload them to the place the feed
URL points at:

```
My App-1.1.0-darwin-arm64.app.tar.gz        the update
My App-1.1.0-darwin-arm64.app.tar.gz.sig    its signature
latest.json                                 the feed
```

In CI, `SEPAL_UPDATER_KEY` can hold the private key instead of
`--update-key`.

What it guarantees:

- An update is installed only if its signature matches the key in the running
  app. A changed archive, or one signed by someone else, is refused and the
  app stays as it is.
- The app never moves to the same or an older version.
- The feed and the archive are fetched over HTTPS only (plain HTTP is accepted
  for `127.0.0.1`, for local testing).
- Bundling refuses a signing key that does not match `updater.pubkey`.

What it does not do yet:

- **Installing is macOS only.** `check()` and `download()` work everywhere;
  `install()` replaces a `.app` bundle.
- **No download progress**, and no partial or delta updates: the whole app is
  downloaded.
- **`install()` rejects in dev** and whenever the app is not running from a
  bundle, or cannot write to where it is installed.
- **A feed for several platforms** is merged when builds of the same version
  write into the same directory; across CI machines you merge the
  `platforms` entries yourself.

`examples/updater` has the full flow with a small UI, and
`scripts/updater-e2e.sh` runs a real update on a Mac: an installed 0.1.0
finds 0.2.0 on a local server, replaces itself and relaunches.

The updater talks to the feed URL and the archive it names, and nothing else.

## Desktop APIs

An app reaches outside its window through the APIs below. All of them are
**off until `permissions` in `sepal.config.ts` allows them**, and the
host enforces that, not JavaScript. A refused call rejects with a message
naming the entry to add.

```ts
permissions: {
	http: ['https://api.example.com'],   // which servers fetch() may talk to
	fs: ['data', 'downloads'],           // which directories the file APIs may use
	dialog: true,
	clipboard: true,
	open: true,
	window: true
}
```

| API | Import | Permission |
|---|---|---|
| `fetch(url, { method, headers, body })` | a global, nothing to import | `http` |
| `readTextFile`, `writeTextFile`, `readFile`, `writeFile`, `exists`, `mkdir`, `readDir`, `remove`, `dir` | `sepal/fs` | `fs` |
| `open`, `save`, `message`, `confirm` | `sepal/dialog` | `dialog` |
| `readText`, `writeText` | `sepal/clipboard` | `clipboard` |
| `open(linkOrPath)` | `sepal/shell` | `open` |
| `setTitle`, `minimize`, `maximize`, `unmaximize`, `isMaximized`, `setDecorations`, `startDragging`, `close` | `sepal/window` | `window` |

```ts
import { writeTextFile, readTextFile } from 'sepal/fs';
import { open } from 'sepal/dialog';

// The app's own data directory; created on first write.
await writeTextFile('settings.json', JSON.stringify(settings), { dir: 'data' });

const response = await fetch('https://api.example.com/items');
const items = await response.json();

// A file the user picks may be read without being listed in permissions.fs.
const path = await open({ filters: [{ name: 'Text', extensions: ['txt', 'md'] }] });
if (path) console.log(await readTextFile(path));
```

How the permissions work:

- **`http`** entries are origins, optionally with a path prefix
  (`https://example.com/api/`) or a wildcard host (`https://*.example.com`).
  `true` allows any. Redirects are followed, and every hop is checked.
- **`fs`** entries are directory names (`data`, `documents`, `downloads`,
  `desktop`, `home`) or absolute paths; `true` allows any path. `..` and
  symlinks cannot lead out of an allowed directory. A path is absolute, or
  relative to a named directory given as `{ dir }`.
- **`open`** opens `http:`, `https:` and `mailto:` links, and files the app may
  read. Every other kind of link is refused.

What to know:

- **`fetch` is the everyday part of the Fetch API**: method, headers, a string
  or byte body, and `text()`, `json()`, `arrayBuffer()`, `bytes()`. `Request`,
  `Headers` and `Response` are there too, because libraries test for them
  (SvelteKit's client checks `instanceof Request` on every fetch). No
  streaming, no `AbortSignal`, no `FormData`.
- **Calls made while the app starts** wait until the window exists.
- **A message or confirm dialog blocks the app** until it is answered.
- **Not run here:** writing to the real clipboard and opening a link in the
  browser (both would have acted on this machine while I worked), and reading
  the clipboard in a real window. `setTitle`, `setDecorations`,
  `startDragging` and `close` are wired to the window but were not observed.
- **Missing:** notifications, menus, tray, global shortcuts, multiple windows,
  window size and position, running other programs, a settings store.

`examples/desktop` has a button for each.

## TypeScript

The examples are TypeScript: `src/main.ts`, `<script lang="ts">` and
`vite.config.ts`, checked with `svelte-check`. The preset picks up `src/main.ts`
or `src/main.js`, whichever exists. Plain JavaScript apps work unchanged.

The packages ship declarations for `mount`, `unmount`, the desktop APIs and
the Vite presets. Their source stays JavaScript with JSDoc; `tsc` checks it and
generates each package's `types/` on `pnpm install`.

- Svelte strips type annotations itself. TypeScript features that emit code
  (`enum`, parameter properties) in a `.svelte` file need `vitePreprocess`,
  passed as `sepal({ svelte: { preprocess: vitePreprocess() } })`.
- An app's `tsconfig.json` includes the `dom` lib because Svelte's element and
  event types need it. That means `window`, `document` and `fetch` type-check
  but do not exist at runtime.
- TypeScript is held at 6.x: `svelte-check` 4.7 does not accept TypeScript 7.

## Layout

```
packages/sepal/           sepal: the runtime, the preset and the command
  src/renderer.js         createRenderer(): node mirror, op buffer, event bubbling
  src/shims.js            console, timers, queueMicrotask for a bare engine
  src/index.js            mount() and the host's entry points (globalThis.__sb)
  src/protocol.js         opcodes shared with the host
  src/host-call.js        asking the host for work that takes time
  src/updater.js          check(), download(), install()
  src/fetch.js            the global fetch
  src/fs.js, dialog.js, clipboard.js, shell.js, window.js   desktop APIs
  src/dom.js              window, document and element methods, for `dom: true`
  src/url.js, text.js     URL, URLSearchParams, TextEncoder, TextDecoder (part of the DOM layer)
  vite/vite.js            sepal/vite, the Vite preset: one app.js (IIFE) + one app.css
  vite/bindings.js        rewrites `bind:` on elements, which Svelte refuses here
  vite/head.js            rewrites `<svelte:head>`, likewise
  vite/app.js             sepal.config.ts, the manifest, the dev window: shared by both presets
  vite/icon.png           the Sepal icon, for apps without one (a copy of assets/brand/sepal-app-icon-1024.png)
  vite/sveltekit.js       sepal/sveltekit, sepalKit(): SvelteKit on this renderer
  cli.js                  the sepal command
  test/                   a host written in JS, for testing without Rust, and the preset's tests
packages/create/          create-sepal
  template/               the seed it copies
packages/host/            @ethercorps/calyx
  index.js                finds this machine's binary
packages/calyx-<platform>/  @ethercorps/calyx-<platform>
  bin/calyx               the binary (built, not committed)
scripts/build-host-package.js   builds the host into packages/calyx-<platform>/bin
scripts/check-versions.js       all packages must share one version
scripts/publish.js              publishes the packed tarballs: order, dist-tags
crates/host/              the `sepal` crate; its binary is calyx
  src/js.rs               QuickJS embedding
  src/ops.rs              applies op batches to blitz-dom
  src/scroll.rs           scrolls the app asks for, and what it is told when one happens
  src/app.rs              app directory, app.json manifest, icon decoding
  src/bundle.rs           macOS .app bundle: Info.plist, .icns
  src/document.rs         SvelteDocument: implements blitz_dom::Document
  src/net.rs              loads data: URLs and files from the app directory
  src/updater.rs          self-update: feed, download, signature, install; and the build side
  src/api/                desktop APIs and the permission checks: http, fs, dialog, open
  src/timers.rs           wakes the event loop for JS timers
  src/window.rs           window + GPU renderer (blitz-shell, Vello Hybrid)
  src/screenshot.rs       CPU rendering to PNG, no display needed
  src/hud.rs              the performance overlay
  src/testing.rs          query(), click(), text() for headless tests
  tests/counter.rs        end-to-end tests against the example bundle
  tests/manifest.rs       manifest parsing and icon decoding
  tests/bundle.rs         bundle layout, Info.plist, .icns contents
  tests/icons.rs          icon and image pixels, resource loading
  tests/sveltekit.rs      a SvelteKit app, by clicking through examples/sveltekit
  tests/router_dom.rs     SvelteKit's router and Bits UI together (examples/router-dom-probe)
  tests/tracker/          the Tracker, a SvelteKit app, screen by screen
  tests/updater.rs        updates against a local server, tampering, the JS API
  tests/desktop.rs        desktop APIs from JS: fetch, files, permissions
crates/blitz-dom/         blitz-dom 0.3.0-beta.4 with nine changes (see PATCH.md)
crates/anyrender_vello_hybrid/   the window's renderer, 0.11.0, with two (see PATCH.md)
examples/counter/         Svelte 5 + Tailwind 4 + TypeScript example
examples/shadcn/          shadcn-svelte components, and a probe for the ones that fail
examples/sveltekit/       a SvelteKit app: links, goto, load, back
examples/tracker/         the Tracker, a SvelteKit app with shadcn-svelte (the benchmark app)
examples/quakes/          live earthquake feeds on a map, a SvelteKit app made from the starter
```

## How it works

```
 Svelte runtime ── renderer.js ──► op buffer ──┐            (JavaScript, QuickJS)
        ▲                                      │ one batch per turn
        │ dispatch(target, type, detail)       ▼
 SvelteDocument ◄── Blitz event driver    apply_batch ──► blitz-dom ──► paint
                                                             (Rust)
```

**Writes are batched.** Svelte calls `createElement`, `insert`, `setText` and so
on. None of those calls leave JS: each appends a few integers to an array. At
the end of a turn the host calls `__sb.take()` once and applies the whole batch
through a single `DocumentMutator`, so Blitz restyles and relayouts once.
Mounting the example (1,000 rows, about 6,000 nodes) is one batch; a counter
click is a batch of 2 ops.

**Reads never cross the boundary.** Svelte walks the tree synchronously
(`getFirstChild`, `getNextSibling`, `getParent`), so each node has a small JS
object with parent and sibling pointers. Unlike a shadow tree that is re-projected
every frame, this mirror is only touched when Svelte touches it.

**Node ids are allocated in JS.** Creating a node needs no return value from the
host, which is what lets creation be batched too.

**Events.** Blitz hit-tests and runs its event driver. For each DOM event the
host checks a set of event types that have listeners; if nobody listens (mouse
moves, typically) JS is not entered at all. Otherwise it calls `__sb.dispatch`
once and the renderer bubbles the event through its own tree, because Svelte
does not delegate or bubble for custom renderers. Handlers run synchronously
inside Blitz's dispatch, so `preventDefault()` works; that is safe because
handlers only write to the op buffer.

**No frame loop.** The event loop waits in the OS. JS timers live in JS; the
host reads the next deadline at the end of each turn and a small thread wakes
the loop when it is due. With no timer pending and no input, nothing runs.

**Memory.** When Svelte drops a subtree, the JS node objects become garbage.
A `FinalizationRegistry` turns that into `DROP` ops and the host frees the Blitz
nodes.

## What was verified, and what was not

Checked on Linux x86-64, in a sandbox with no display:

| | |
|---|---|
| Renderer against Svelte 5.59.1 in an engine with no DOM globals | `pnpm run test:js` |
| Real bundle → QuickJS → blitz-dom 0.3.0-beta.4 → style and layout | `tests/counter.rs` |
| Tailwind 4.3 stylesheet (incl. `oklch()` colours, `@layer`) lays out and paints correctly | layout assertions + `screenshot.png`, which `tests/counter.rs` writes |
| Clicks through Blitz hit testing, bubbling, `{#if}`, keyed `{#each}` reorder | `counter_app` |
| Inline SVG and Lucide icons in hex and Tailwind colours, recolouring after a click, `<img>` from a data: URL, a file and an import | `tests/icons.rs` (pixels) + `docs/icons-probe.png` |
| Resources: only data: URLs and files inside the app directory load; bad URLs do not crash | `tests/icons.rs` |
| SvelteKit's router in the Tracker: links, `goto`, a redirect, nested layouts that stay mounted, `load` with params, errors inside a layout, an unknown address as a 404 | `tests/tracker/` |
| SvelteKit's router with Bits UI: twenty navigations leave no popups or nodes behind, a link inside a Dialog navigates, a click in a Select list picks | `tests/router_dom.rs` |
| Server files (`+page.server.*`, `+server.*`, `hooks.server.*`, `*.remote.*`) stop a `sepalKit()` build; `src/routes` without `src/main` stops a `sepal()` build | `test/sveltekit.test.js`, `test/preset.test.js` |
| Updater: signatures, feed rules, a bundle replaced from a local server, tampered and foreign-signed updates refused, the JS API | `tests/updater.rs` |
| Updater on a real bundle: 0.1.0 finds 0.2.0, verifies, replaces itself, relaunches; code signature still valid | `scripts/updater-e2e.sh` (macOS) |
| Desktop APIs from JavaScript: fetch (methods, bodies, redirects, errors), files (text, bytes, directories), URL and path scopes, every call refused without permission, a slow request not blocking the UI | `tests/desktop.rs` |
| Desktop APIs on a real Mac: HTTPS fetch to api.github.com, a file in the app data directory, confirm and file-open dialogs appearing, maximise, minimise | example run with `VITE_AUTO=<button>` |
| shadcn-svelte components from its CLI: nine render and react, `bind:this` reaches the element | `tests/shadcn.rs`, `examples/shadcn/probe.mjs` |
| Bits UI components on the DOM layer: Switch and Checkbox (bound value reaches the page), Tabs, Accordion, Dialog (opens over the page, closes on a click outside) | `tests/shadcn.rs`, `test/dom.test.js` |
| Measuring: a script sees what it just changed, an Accordion slides open to its measured height, a Select opens under its trigger in the same turn and picks an item, a Tooltip is centred over what is hovered, `ResizeObserver`, `innerWidth` and `resize` follow the window, an idle turn reports nothing | `tests/shadcn.rs`, `test/dom.test.js` |
| The same Select and Accordion in a real window on macOS, by scripted clicks and the trace: they open and close, a click takes 2 to 4 ms, then no more redraws | `SEPAL_TRACE=1` with `--watch` |
| Exit animations: an Accordion slides shut and is hidden after, a Dialog is still on the page while it fades | `tests/shadcn.rs` |
| Keyboard: Tab, Enter and Space on buttons, typing and editing keys in a bound input, a Select driven by arrows, Enter and Escape, a Dialog that traps Tab and returns the focus, focus ring for keyboard only | `tests/shadcn.rs`, `tests/counter.rs`, `test/dom.test.js` |
| The same keys in a real window on macOS, scripted (`key`, `type` on stdin) and traced | `SEPAL_TRACE=1` with `--watch` |
| An attribute change restyles descendants and siblings, with and without sibling selectors in play | `tests/restyle.rs` |
| Children that are added, removed or moved are styled, and so is what depends on their position (`:first-child`, `:nth-child()`, `a + b`, `:empty`) | `tests/restyle.rs` |
| `:has()`: follows an attribute, a child that arrives or leaves, and the focus, for a parent, an ancestor, what is under it, and siblings on either side | `tests/restyle.rs` |
| The window's GPU renderer draws what the CPU screenshots show: the same page differs by at most 2/255 per channel | `drive … gpushot:` against `shot:` (needs a GPU; by hand) |
| A SvelteKit app built with `sepalKit()`: a link, `goto`, a parameter with `load`, `$app/state`, back twice, component styles, a static image by root path, `<svelte:head>` titles | `tests/sveltekit.rs`, `test/head.test.js` |
| SvelteKit in dev: an edit rebuilds, `dist/` has the change, the host is told to reload | by hand, with a stand-in host |
| `URL`, `URLSearchParams`, `TextEncoder`, `TextDecoder` agree with Node's on 231 URLs and on text cut at every byte | `test/url.test.js`, `test/text.test.js` |
| A promise rejected with nobody listening is reported as `Uncaught (in promise)`; one that is handled is not | `tests/counter.rs`, on both engines |
| A button in a line of text is clicked where it is drawn | `tests/shadcn.rs` |
| A script that never returns is stopped after 10 s | unit test in `js.rs` |
| Idle turns do nothing; unused event types never enter JS | `idle_turns_do_no_work` |
| Timer wake-ups, and silence when no timer is running | `timers_wake_the_event_loop` |
| Removed nodes are freed on the host after GC | `removed_nodes_are_freed` |

Numbers from the dev-profile build on 2 shared cores, so treat them as
rough: mount of 6,000 nodes about 290 ms (most of it QuickJS running Svelte),
first style and layout 47 ms, click to updated layout 2 ms.

Checked on macOS 27 (Apple M4 Pro, arm64, rustc 1.95):

| | |
|---|---|
| Everything in the table above | `pnpm test` |
| CPU screenshot with macOS system fonts | `--screenshot`, inspected by eye |
| The window opens (Vello Hybrid), 800×600 logical, no errors or panics | `pnpm start` |
| Idle with the timer stopped: 0.0% CPU, 0 idle wake-ups over 8 s, 84 MB | `top -stats cpu,idlew,mem` |
| Manifest → host: title, size and icon parsed; bad icons are errors, not panics | `tests/manifest.rs` |
| The window opens at the size the manifest asks for | window bounds from WindowServer |
| Bundle: `plutil` and `iconutil` accept the plist and `.icns`, `codesign --verify --strict` passes, the app launches from `open`, macOS reports the name "Counter" and shows the icon for the bundle | `pnpm run bundle`, macOS tools |
| A fresh project made by the packed create script and installed from the package tarballs, no `SEPAL_HOST`: type-check, build, render | `pnpm run test:template` |
| Dev runs from a bundle: macOS reports the name "Counter" and its bundle id | `pnpm dev`, `lsappinfo` |
| Developer ID signing: `codesign --verify --strict` passes, hardened runtime flag set, the app launches | `--sign` with a real identity |
| Dev reload: edit → reload in ~130 ms, broken builds keep the old one, Vite and host stop together | `pnpm dev`, read from the host log |

**Not verified:**

- **What the window shows and how it reacts.** On macOS it opens and idles
  correctly, but painting, clicks, wheel scrolling, hover styles, resizing and
  CPU with the timer running have not been confirmed by eye yet.
- **The Dock icon and window title on macOS.** The code runs without errors,
  but nobody has looked at the Dock yet.
- Windows builds, and the window on Linux. That includes the window icon on
  Windows and X11.
- Real key presses. Keyboard tests and the scripted keys start at the
  document; nobody has typed into a window, so how winit and AppKit deliver
  keys (and Backspace, which macOS sends as a command) is unconfirmed. IME has
  not been tried at all.

## Performance

Measured on an M-series Mac, the shadcn rows in a window with
`SEPAL_TRACE=1` (which prints how long each input took and how many
layouts it needed), the counter rows headless with the `drive` example. Time
from the press to the page being ready to paint:

| | |
|---|---|
| Counter: click the button (one text node) | 1 ms |
| Counter: select a row among 1000 | 5 ms |
| Counter: shuffle 1000 rows | 36 ms |
| shadcn: toggle an Accordion | 3 ms |
| shadcn: open a Dialog (199 ops) | 8 ms |
| shadcn: open a Select (291 ops, placed by Floating UI) | 22 ms |
| Start to first layout: counter (3000 nodes) / shadcn page | 0.17 s / 0.10 s |

What the time is spent on, from sampling:

- **JavaScript.** The 1000-row cases are almost entirely Svelte running in
  QuickJS, which interprets and has no JIT. Expect code that loops over
  thousands of items to run several times slower than in a browser.
- **Layouts forced by measuring.** A library that writes a style and measures
  again makes the host lay out each time, as a browser would. Opening the
  Select does that 7 times, 9 ms in all.
- **A frame.** While anything animates, Blitz paints the whole window every
  refresh. Half of that used to be rasterising every glyph again; the
  renderer's glyph atlas is now on (a patch to the vendored
  `anyrender_vello_hybrid`), which cut the time spent painting by half. A
  page with an endless animation, such as a `Skeleton`, still never idles.
- **The engine.** QuickJS is an interpreter. The host can be built on V8
  instead (`cargo build --features engine-v8`, experimental): long lists get
  about 3.5 times faster and components about 1.5 times once warm, for a
  binary of 62 MB instead of 24. `HANDOVER.md` has the table.

### The performance overlay

Press **Cmd+Option+P** in the window (Ctrl+Alt+P on Windows and Linux) to
show or hide a box of numbers in the top-right corner. It can also start
shown, with `SEPAL_HUD=1` in the host's environment, and in dev mode
the line `hud` on the host's stdin toggles it. The host draws it; the app
cannot see or restyle it, and clicks go through it.

```
 60 fps   frame 4.2 / 9.1 ms
CPU 12%   GPU (system) 8%
layouts 2/s   QuickJS
mem 142 MB   peak 160 MB
nodes 3120   turn 3.1 ms, 42 ops
```

| | |
|---|---|
| fps | frames painted in the last second. Nothing is painted unless something changed, so an idle app reads 1: the overlay's own refresh. |
| frame | average and longest time the UI thread spent on one frame (layout, paint and handing it to the GPU), in the last second |
| CPU | the process's CPU time over the last second, all threads: 100% is one core |
| GPU (system) | macOS only: how busy the GPU is, as the driver reports it to IOKit. **For the whole machine,** not this app: another app playing video shows up here. macOS has no per-app figure outside Instruments. |
| layouts | layouts the host did between frames, for the app's measurements and scrolls. The layout before each paint is part of "frame". |
| QuickJS / V8 | the JavaScript engine this host was built with |
| mem, peak | macOS: the physical footprint (Activity Monitor's "Memory"), now and the highest since start. Linux: resident memory. Not shown on Windows. |
| nodes | nodes the app has made that the host holds |
| turn | the last turn that changed the page: from the input (or timer) to the page being ready to paint, and the number of ops it applied |

Hidden, the overlay costs nothing. Shown, it wakes the window once a second
to refresh, which is the one repaint it causes. With `SEPAL_TRACE=1`
each refresh is also printed to stderr.

## Limitations

- `transition:`, `animate:` and `<svelte:window>` are rejected by Svelte for
  custom renderers. `<svelte:head>` is too, but the preset rewrites it: its
  `<title>` sets `document.title` and the rest is dropped. So is `bind:` on elements, but the preset
  rewrites it: `bind:this` and `bind:value` (on inputs) work, and other
  element bindings compile and warn that they do nothing.
- Desktop APIs cover fetch, files, dialogs, clipboard, opening links and basic
  window control. See "Desktop APIs" for what is missing.
- Images and other resources load from data: URLs and the app directory only; no network.
- `requestAnimationFrame` is a 16 ms timer, not tied to the display.
- Measurements (`dom: true` only) leave out CSS transforms, and inside an
  event handler are as of the last layout. See "Measuring the page".
- A script that runs for 10 seconds without returning is stopped.
- An element is put in the SVG namespace by its name alone. `a`, `title`,
  `style` and `script` are always HTML, also inside an `<svg>`.
- Blitz itself is beta. Its status page lists `position: sticky`, container
  queries, 3D transforms and `overflow: auto` as missing; `backdrop-filter`
  needs its Skia backend.
- One window. Packaging for macOS only; notarisation is implemented but has never been run. Dev reload resets state (no HMR).
- The Svelte custom renderer API shipped in 5.59.0 and is days old. Blitz and
  the renderer crates are pinned to exact versions for the same reason.

## Next steps

1. **Real input in a window.** Key presses have only been sent as scripts, at
   the document; next are presses through winit and AppKit, IME, and
   clipboard shortcuts in text fields.
2. **Other platforms.** Open the window on Linux, which builds and passes its
   tests headless but has never been run with a display. Windows is on hold:
   the host builds and most tests pass (see HANDOVER.md). Then packaging
   beyond a macOS `.app`: a `.dmg`, and Windows and Linux bundles, and a real
   notarisation run.
3. **Links at the next frame.** `requestAnimationFrame` at the next frame
   the host draws, and no extra turn for a `setTimeout(0)` queued from a
   frame callback: SvelteKit waits for both before it follows a link.
4. **The native layer.** A JavaScript backend thread (the stem) and native
   plugins (grafts).
5. **More desktop APIs**: notifications, menus, a tray icon, global
   shortcuts, more than one window, a settings store.
6. **Memory while drawing.** An idle Tracker sits at about 100 MB, but after
   scrolling 1,000 rows it holds 305 MB of RAM and 220 MB of GPU memory
   ([compare.md](compare/results/compare.md)); a virtual list, and letting go
   of GPU memory sooner, are the places to start.
7. **Later**: state-preserving HMR, if the full reload turns out not to be
   enough; compiling the bundle with scriptc instead of running QuickJS (only
   `js.rs` should need to change).

## Licence

MIT, see `LICENSE`.

The host binary statically links Stylo and its companion crates (the CSS
engine), which are MPL-2.0. That does not change the licence of this code, but
a distributed binary has to say so and point at their source:
`THIRD-PARTY-LICENSES.txt` in the host package lists every crate in the
binary with its licence and where its source is, followed by the licence
texts. `pnpm run build:host` writes it.
