# Changelog

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
