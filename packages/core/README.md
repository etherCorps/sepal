# @ethercorps/svelte-blitz

The runtime of [svelte-blitz](https://github.com/etherCorps/svelte-blitz): Svelte apps in a native window, without
a webview and without Node at run time. Svelte components are compiled with
Svelte's custom renderer API and drive [Blitz](https://github.com/DioxusLabs/blitz),
a Rust HTML/CSS engine; the JavaScript runs in an embedded QuickJS.

Start a project with the template rather than by installing this directly:

```sh
pnpm create @ethercorps/svelte-blitz my-app
```

What is in here:

| Import | |
|---|---|
| `@ethercorps/svelte-blitz` | `mount`, `unmount` |
| `…/fs`, `…/dialog`, `…/clipboard`, `…/shell`, `…/window` | desktop APIs, each behind a permission in `app.config.ts` |
| `…/updater` | self-update |
| `…/dom` | a small DOM for component libraries written for the web (`dom: true`) |
| `…/renderer`, `…/protocol` | what the Vite preset and the host are built against |

`fetch`, `console` and timers are globals. Everything else is in the
[documentation](https://github.com/etherCorps/svelte-blitz#readme).

MIT
