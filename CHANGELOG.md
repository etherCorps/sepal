# Changelog

## Unreleased

- **`fetch` in SvelteKit apps.** `Request`, `Headers` and `Response` are
  globals now, next to `fetch`, which also takes a `Request`. SvelteKit's
  client checks `instanceof Request` on every fetch, so until now every
  `fetch` in a SvelteKit app failed with "Request is not defined".
- **Fonts and other `url()`s in SvelteKit apps.** The adapter merges
  SvelteKit's stylesheets into `app.css`; their relative `url()`s now point
  where the files are, so `@font-face` fonts load.
- **`#lib` in the SvelteKit starter.** SvelteKit 3 replaced `$lib` with
  `#lib`, which a project declares in package.json's `imports`. The
  starter now does, and its `src/lib/index.ts` says how to import it.

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
