# svelte-blitz

Svelte apps in a native window, without a webview and without Node at runtime.

Svelte components are compiled with Svelte's custom renderer API. Instead of the
browser DOM they drive [Blitz](https://github.com/DioxusLabs/blitz), a Rust
HTML/CSS engine (Stylo for CSS, Taffy for layout, Parley for text, Vello for
painting). The JavaScript runs in an embedded QuickJS-ng. Styling is ordinary
CSS, so Tailwind works as it does on the web.

This is a starter covering the first three milestones: static render, events and
reactivity, Tailwind. It is small enough to read in one sitting (about 700 lines
of JS and 1,200 of Rust).

![The example app rendered by Blitz](screenshot.png)

## Run it

Requires Node 22+, a recent stable Rust (built here with 1.97) and, on Linux, the usual windowing libraries
(`libxkbcommon`, X11 or Wayland dev packages, a Vulkan driver).

```sh
pnpm install
pnpm test              # type checks, JS renderer test, then the headless end-to-end tests
pnpm run check         # type checks only (tsc for the package, svelte-check for the examples)
pnpm run screenshot    # render the example to screenshot.png on the CPU
pnpm start             # open the example in a window
pnpm dev               # the same, reloading on every save
```

## Start a project

```sh
pnpm create @ethercorps/svelte-blitz my-app   # TypeScript, Tailwind, file-based routes, icon, manifest, static/
cd my-app
pnpm install
pnpm dev                        # opens the app, reloads on save
pnpm start                      # build once and open it
pnpm bundle                     # my-app.app
pnpm check                      # svelte-check
```

**Nothing is published yet.** Until it is, create a project from this
repository with `node packages/create/index.js ../my-app` and install the
packages from tarballs made with `pnpm pack`; `packages/create/template.test.js`
does exactly that.

## Packages

| Package | What it is | Directory |
|---|---|---|
| `@ethercorps/svelte-blitz` | The runtime: `mount`, the renderer, the op protocol. No Node code. | `packages/core` |
| `@ethercorps/svelte-blitz-vite` | The Vite preset (build, app manifest, `static/`, dev reload) and the file-based router. | `packages/vite` |
| `@ethercorps/svelte-blitz-cli` | The `svelte-blitz` command. | `packages/cli` |
| `@ethercorps/create-svelte-blitz` | The starter template. | `packages/create` |
| `@ethercorps/svelte-blitz-host` | The host binaries, and the lookup that picks one. | `packages/host` |

They are released together under one version (`pnpm run check` fails if they
drift). An app depends on the first, and has the Vite preset and the CLI as
dev dependencies.

```
svelte-blitz run [dist]
svelte-blitz bundle [dist] [out-dir] [--sign <identity>] [--notarize <keychain-profile>]
svelte-blitz screenshot <out.png> [dist] [WIDTHxHEIGHT]
```

### Where the host binary comes from

One package, `@ethercorps/svelte-blitz-host`, holds the binary for every
platform under `bin/<platform>/`, so nobody who installs from npm needs Rust.
The Vite preset and the CLI ask that package for the path;
`SVELTE_BLITZ_HOST` overrides it with a host you built yourself.

Only macOS arm64 exists so far: 22.8 MB, about 10 MB packed
(`pnpm run build:host` builds it into `packages/host/bin/darwin-arm64`). Intel
macOS, Linux and Windows need a CI build each, and every one adds roughly that
much to what every user downloads. If that grows uncomfortable, the package
can switch to fetching only the matching binary on first use; nothing outside
it would change.

## Dev: reload on save

`vite build --watch` is the dev command (`pnpm dev` in this repository builds
the host first and points `SVELTE_BLITZ_HOST` at it). The preset starts the
host after the first build and tells it to reload after each later one. The window stays open and keeps its size,
position and page scroll; the app itself starts over, so component state is
lost. This is a live reload, not Svelte HMR.

- A build error leaves the last good build on screen; so does a bundle that
  throws while it loads. Fix and save again.
- Closing the window ends Vite. Stopping Vite closes the window.
- Changes to `app.json` (title, size, icon) need a restart.
- On macOS the host runs out of a throwaway bundle in
  `node_modules/.svelte-blitz/`, so the Dock and the app switcher show the
  app's name and icon in dev too. This needs `app.name` in the manifest.
- There is no file watcher and no polling in the host: Vite writes one line to
  the host's stdin per rebuild. `svelte-blitz-host --watch dist` run by hand
  reloads when you press Enter.


## App config

An app describes itself in `svelte-blitz.config.ts`, next to `vite.config.ts`:

```ts
import { defineConfig } from '@ethercorps/svelte-blitz-vite';

export default defineConfig({
	app: {
		name: 'Counter',
		id: 'com.example.counter',
		icon: 'icon.png', // PNG, 1024×1024 recommended, relative to the project root
		window: { title: 'Svelte on Blitz', width: 800, height: 600 }
		// version: defaults to the one in package.json
		// updater: see "Updates"
	},
	permissions: {
		// what the app may do outside its window; see "Desktop APIs"
	}
	// router: see "Your own router"
});
```

That leaves `vite.config.ts` as a list of plugins:

```ts
export default defineConfig({
	plugins: [tailwindcss(), svelteBlitz()]
});
```

The file can also be `.js` or `.mjs`, and can export a function, which gets
Vite's `{ mode, command }`. Anything passed to `svelteBlitz({ … })` directly
overrides the file, key by key. In watch mode, edits to `app` and
`permissions` are picked up on the next build; a change to `router` needs
Vite restarted.

The build writes `dist/app.json` and copies the icon to `dist/icon.png`. Point
the host at the directory and it picks them up: `svelte-blitz-host dist`.
Every field is optional; `svelte-blitz-host app.js app.css` still works and
runs without a manifest.

| Platform | Where the icon shows |
|---|---|
| macOS | Dock, set at runtime through AppKit. Leave a transparent margin around the artwork (an 824px tile on the 1024px canvas) or it looks larger than its neighbours. |
| Windows, X11 | Window icon through winit. Not run yet. |
| Wayland | Nothing at runtime: compositors take the icon from a `.desktop` file. Not done yet. |

A missing or undecodable icon stops the app with an error naming the file.

On macOS `name` also becomes the process name, which is what the menu bar
items ("Quit Counter") use. Run straight from the host binary, the Dock label,
the app switcher and Activity Monitor still say `svelte-blitz-host`: macOS takes
those from an app bundle. Build one to get them right.

## Bundle a macOS app

```sh
pnpm run bundle        # examples/counter/bundle/Counter.app
svelte-blitz-host --bundle dist [out-dir]
```

```
Counter.app/Contents/
  Info.plist           name, id, version, icon
  MacOS/Counter        the host binary that did the bundling
  Resources/           everything from dist/, plus AppIcon.icns
```

The manifest needs a `name`. `id` (default `app.svelte-blitz.<name>`) and
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

svelte-blitz bundle dist --sign "Developer ID Application: Your Name (TEAMID)" --notarize my-profile
```

`--sign` signs with the hardened runtime and a secure timestamp; the host
needs no entitlements. `--notarize` zips the app, submits it with `notarytool`,
waits for the result and staples the ticket to the bundle. `--sign` alone has
been run here with a real Developer ID and the result launches; Gatekeeper
then reports "Unnotarized Developer ID" until it is notarised. **`--notarize`
has not been run**: it uploads to Apple and needs your credentials.

## Routing

Pages are files in `src/routes`, with SvelteKit's conventions. A project that
has that directory needs no `main.ts`: the router is the entry.

```
src/routes/
  +layout.svelte            wraps every page; stays mounted while pages change
  +layout.ts                load() for the layout
  +page.svelte              /
  +error.svelte             shown for errors and unknown paths
  about/+page.svelte        /about
  items/+page.svelte        /items
  items/[id]/+page.svelte   /items/42          params.id === '42'
  items/[id]/+page.ts       load() for that page
  docs/[...path]/+page.svelte   /docs/a/b      params.path === 'a/b'
  (account)/+layout.svelte  a group: a layout without a path segment
  (account)/settings/+page.svelte   /settings
```

```ts
// src/routes/items/[id]/+page.ts
import { error } from '@ethercorps/svelte-blitz-vite/router';
import { items } from '$lib/items';

export async function load({ params, parent }) {
	const item = items.find((item) => item.id === params.id);
	if (!item) error(404, `No item ${params.id}`);
	return { item, ...(await parent()) };
}
```

```svelte
<!-- src/routes/items/[id]/+page.svelte -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let { data, params } = $props();
</script>

<h1>{data.item.name}</h1>
<a href="/items">all items</a>
<button onclick={() => goto('/')}>home</button>
<p>{page.url.pathname}</p>
```

What works as in SvelteKit:

- `+page.svelte`, `+layout.svelte` (nested, with `{@render children()}`),
  `+error.svelte`, `[param]`, `[...rest]`, `(group)`.
- `load` in `+page.js`/`.ts` and `+layout.js`/`.ts`, sync or async, with
  `params`, `url`, `route` and `parent()`. The result arrives as the `data`
  prop, merged with the layouts' data. The old page stays up until the new
  one's loads are done.
- `<a href="/path">` navigates. `goto()` from `$app/navigation`, `page` from
  `$app/state`, and the `$lib` alias.
- `error(status, message)` and `redirect(status, location)` inside `load`,
  imported from `@ethercorps/svelte-blitz-vite/router`.

What is different, because there is no browser and no server:

- **The location lives in memory.** There is no address bar and no History
  API. `back()` and `forward()` come from `$app/navigation` instead of
  `history`.
- **`page.url` is a plain object** with `pathname`, `search`, `hash`, `href`
  and `searchParams.get()/has()`. The engine has no `URL` class.
- **No server files.** `+page.server.*`, `+layout.server.*` and `+server.*`
  are build errors. So are optional parameters (`[[x]]`) and matchers
  (`[x=m]`).
- **No generated `./$types`.** Type `data` and `params` yourself.
- **Every `load` runs on every navigation**; there is no `invalidate`, and no
  `beforeNavigate`/`afterNavigate`.
- **All pages are in one bundle.** There is no code splitting.

`examples/routing` exercises all of it.

### Your own router

The built-in router is the default, not a requirement. The preset's `router`
option replaces it or turns it off:

```ts
svelteBlitz({ router: false })                          // no routing; src/main.ts is the entry
svelteBlitz({ router: { entry: 'src/my-router.ts' } })  // your router is the entry
svelteBlitz({ router: { dir: 'src/pages' } })           // built-in router, other directory
```

A router of your own is a module that mounts the app. It can use the route
table the preset generates from the routes directory, or ignore it:

```ts
// src/my-router.ts
import { mount } from '@ethercorps/svelte-blitz';
import { routes, fallback } from 'virtual:svelte-blitz/routes';
import Shell from './Shell.svelte';

// Each route: { id, pattern, names, layouts, page, error, error_depth }.
// `page` and each layout are { component, load }.
mount(Shell, { props: { routes, fallback } });
```

With a custom entry the built-in `$app/navigation` and `$app/state` are not
set up; your router exports its own API. The `Route` and `RouteNode` types
come from `@ethercorps/svelte-blitz-vite/router`.

Running SvelteKit itself on this renderer is not practical. Its client
router (3,800 lines in 3.0.1) is written against `location`, `history`,
`document`, `fetch` and dynamic `import()`, mounts its root without a
renderer option, and its build expects a server pass.

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
svelte-blitz signer generate ~/.keys/my-app.key
# prints the public key
```

The file is the private key. Keep it secret and backed up: updates signed
with any other key are refused by every copy of the app already installed.

**2. Tell the app where updates come from**, in `svelte-blitz.config.ts`.

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
	import { check } from '@ethercorps/svelte-blitz/updater';

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
svelte-blitz bundle --sign "Developer ID Application: …" --notarize my-profile \
  --update-key ~/.keys/my-app.key --update-notes "What changed"
```

Next to the `.app` this writes three files. Upload them to the place the feed
URL points at:

```
My App-1.1.0-darwin-arm64.app.tar.gz        the update
My App-1.1.0-darwin-arm64.app.tar.gz.sig    its signature
latest.json                                 the feed
```

In CI, `SVELTE_BLITZ_UPDATER_KEY` can hold the private key instead of
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
**off until `permissions` in `svelte-blitz.config.ts` allows them**, and the
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
| `readTextFile`, `writeTextFile`, `readFile`, `writeFile`, `exists`, `mkdir`, `readDir`, `remove`, `dir` | `@ethercorps/svelte-blitz/fs` | `fs` |
| `open`, `save`, `message`, `confirm` | `@ethercorps/svelte-blitz/dialog` | `dialog` |
| `readText`, `writeText` | `@ethercorps/svelte-blitz/clipboard` | `clipboard` |
| `open(linkOrPath)` | `@ethercorps/svelte-blitz/shell` | `open` |
| `setTitle`, `minimize`, `maximize`, `unmaximize`, `isMaximized`, `setDecorations`, `startDragging`, `close` | `@ethercorps/svelte-blitz/window` | `window` |

```ts
import { writeTextFile, readTextFile } from '@ethercorps/svelte-blitz/fs';
import { open } from '@ethercorps/svelte-blitz/dialog';

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
  or byte body, and `text()`, `json()`, `arrayBuffer()`, `bytes()`. No
  streaming, no `AbortSignal`, no `Request`/`Headers`/`FormData` classes.
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

The packages ship declarations for `mount`, `unmount`, the Vite preset and
the router. Their source stays JavaScript with JSDoc; `tsc` checks it and
generates each package's `types/` on `pnpm install`.

- Svelte strips type annotations itself. TypeScript features that emit code
  (`enum`, parameter properties) in a `.svelte` file need `vitePreprocess`,
  passed as `svelteBlitz({ svelte: { preprocess: vitePreprocess() } })`.
- An app's `tsconfig.json` includes the `dom` lib because Svelte's element and
  event types need it. That means `window`, `document` and `fetch` type-check
  but do not exist at runtime.
- TypeScript is held at 6.x: `svelte-check` 4.7 does not accept TypeScript 7.

## Layout

```
packages/core/            @ethercorps/svelte-blitz: the runtime
  src/renderer.js         createRenderer(): node mirror, op buffer, event bubbling
  src/shims.js            console, timers, queueMicrotask for a bare engine
  src/index.js            mount() and the host's entry points (globalThis.__sb)
  src/protocol.js         opcodes shared with the host
  src/host-call.js        asking the host for work that takes time
  src/updater.js          check(), download(), install()
  src/fetch.js            the global fetch
  src/fs.js, dialog.js, clipboard.js, shell.js, window.js   desktop APIs
  test/                   a host written in JS, for testing without Rust
packages/vite/            @ethercorps/svelte-blitz-vite: preset and router
  src/vite.js             Vite preset: one app.js (IIFE) + one app.css
  src/routes.js           reads src/routes into a route table (build time)
  src/routing.svelte.js   the in-memory router: page, goto, back, load, errors
  src/Router.svelte       renders layouts around the current page
  src/router.js           the router's public API
packages/cli/             @ethercorps/svelte-blitz-cli: the svelte-blitz command
packages/create/          @ethercorps/create-svelte-blitz
  template/               the starter it copies
packages/host/            @ethercorps/svelte-blitz-host
  index.js                finds the host binary
  bin/<platform>/         the binaries (built, not committed)
scripts/build-host-package.js   builds the host into packages/host/bin
scripts/check-versions.js       all packages must share one version
crates/host/              the `svelte-blitz-host` crate
  src/js.rs               QuickJS embedding
  src/ops.rs              applies op batches to blitz-dom
  src/app.rs              app directory, app.json manifest, icon decoding
  src/bundle.rs           macOS .app bundle: Info.plist, .icns
  src/document.rs         SvelteDocument: implements blitz_dom::Document
  src/net.rs              loads data: URLs and files from the app directory
  src/updater.rs          self-update: feed, download, signature, install; and the build side
  src/api/                desktop APIs and the permission checks: http, fs, dialog, open
  src/timers.rs           wakes the event loop for JS timers
  src/window.rs           window + GPU renderer (blitz-shell, Vello Hybrid)
  src/screenshot.rs       CPU rendering to PNG, no display needed
  src/testing.rs          query(), click(), text() for headless tests
  tests/counter.rs        end-to-end tests against the example bundle
  tests/manifest.rs       manifest parsing and icon decoding
  tests/bundle.rs         bundle layout, Info.plist, .icns contents
  tests/icons.rs          icon and image pixels, resource loading
  tests/routing.rs        file-based routing, by clicking through examples/routing
  tests/updater.rs        updates against a local server, tampering, the JS API
  tests/desktop.rs        desktop APIs from JS: fetch, files, permissions
crates/blitz-dom/         blitz-dom 0.3.0-beta.4 with two fixes (see PATCH.md)
examples/counter/         Svelte 5 + Tailwind 4 + TypeScript example
examples/routing/         file-based routes: layouts, params, load, errors
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
| Tailwind 4.3 stylesheet (incl. `oklch()` colours, `@layer`) lays out and paints correctly | layout assertions + `screenshot.png` |
| Clicks through Blitz hit testing, bubbling, `{#if}`, keyed `{#each}` reorder | `counter_app` |
| Inline SVG and Lucide icons in hex and Tailwind colours, recolouring after a click, `<img>` from a data: URL, a file and an import | `tests/icons.rs` (pixels) + `docs/icons-probe.png` |
| Resources: only data: URLs and files inside the app directory load; bad URLs do not crash | `tests/icons.rs` |
| File-based routing: links, params, load data, groups, rest parameters, redirects, error pages, back/forward, slow and overtaken loads | `tests/routing.rs`, `test/routes.test.js` |
| Updater: signatures, feed rules, a bundle replaced from a local server, tampered and foreign-signed updates refused, the JS API | `tests/updater.rs` |
| Updater on a real bundle: 0.1.0 finds 0.2.0, verifies, replaces itself, relaunches; code signature still valid | `scripts/updater-e2e.sh` (macOS) |
| Desktop APIs from JavaScript: fetch (methods, bodies, redirects, errors), files (text, bytes, directories), URL and path scopes, every call refused without permission, a slow request not blocking the UI | `tests/desktop.rs` |
| Desktop APIs on a real Mac: HTTPS fetch to api.github.com, a file in the app data directory, confirm and file-open dialogs appearing, maximise, minimise | example run with `VITE_AUTO=<button>` |
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
| A fresh project made by the packed create script and installed from the package tarballs, no `SVELTE_BLITZ_HOST`: type-check, build, render | `pnpm run test:template` |
| A router of the app's own as the entry, and routing switched off | `packages/vite/test/custom-router.test.js` |
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
- Text input, IME, focus and keyboard events. The event payloads are wired
  (`key`, `code`, `value`) but no test types into a field.

## Limitations

- `bind:`, `transition:`, `animate:`, `<svelte:window>`, `<svelte:head>` are
  rejected by Svelte for custom renderers. Use `value={x}` + `oninput`.
- Desktop APIs cover fetch, files, dialogs, clipboard, opening links and basic
  window control. See "Desktop APIs" for what is missing.
- Images and other resources load from data: URLs and the app directory only; no network.
- `requestAnimationFrame` is a 16 ms timer, not tied to the display.
- Capture-phase listeners are treated as bubble-phase.
- An element is put in the SVG namespace by its name alone. `a`, `title`,
  `style` and `script` are always HTML, also inside an `<svg>`.
- Blitz itself is beta. Its status page lists `position: sticky`, container
  queries, 3D transforms and `overflow: auto` as missing; `backdrop-filter`
  needs its Skia backend.
- One window. Packaging for macOS only; notarisation is implemented but has never been run. Dev reload resets state (no HMR).
- The Svelte custom renderer API shipped in 5.59.0 and is days old. Blitz and
  the renderer crates are pinned to exact versions for the same reason.

## Next steps

1. Run `pnpm start` on a machine with a display and fix whatever the window turns up.
2. Text input: a controlled `<input>` test, then IME.
3. Host modules: `fetch`, clipboard, window controls.
4. State-preserving HMR, if the full reload turns out not to be enough.
5. A virtual list component, then packaging per OS.
6. Try compiling the bundle with scriptc and replacing QuickJS. Only `js.rs`
   should need to change; the protocol is engine-agnostic.

## Licence

MIT, see `LICENSE`.

The host binary statically links Stylo and its companion crates (the CSS
engine), which are MPL-2.0. That does not change the licence of this code, but
a distributed binary has to say so and point at their source; a third-party
licence file generated at release time is still to be added.
