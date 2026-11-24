# sepal

[Sepal](https://github.com/etherCorps/sepal): Svelte apps in a native window, without
a webview and without Node at run time. Svelte components are compiled with
Svelte's custom renderer API and drive [Blitz](https://github.com/DioxusLabs/blitz),
a Rust HTML/CSS engine; the JavaScript runs in an embedded QuickJS.

This one package is the runtime, the Vite preset and the `sepal` command.
Start a project with the starter rather than by installing it directly:

```sh
pnpm create sepal my-app
```

What is in here:

| Import | |
|---|---|
| `sepal` | `mount`, `unmount` |
| `sepal/fs`, `sepal/dialog`, `sepal/clipboard`, `sepal/shell`, `sepal/window` | desktop APIs, each behind a permission in `sepal.config.ts` |
| `sepal/updater` | self-update |
| `sepal/dom` | a small DOM for component libraries written for the web (`dom: true`) |
| `sepal/renderer`, `sepal/protocol` | what the Vite preset and the host are built against |
| `sepal/vite` | the Vite preset, `sepal()`, and `defineConfig` for `sepal.config.ts` |
| `sepal/sveltekit` | `sepalKit()`, which takes the place of both `sveltekit()` and the adapter |
| `sepal/router`, `sepal/navigation`, `sepal/state` | the file-based router (`src/routes`, `$app/navigation`, `$app/state`) |
| `sepal/ambient` | types for `$app/*`; add it to `types` in tsconfig.json |

The preset compiles Svelte for the Blitz renderer, writes the one `app.js` and
`app.css` the host loads, reads `sepal.config.ts`, and in `vite build --watch`
opens the app and reloads it on save.

```ts
// vite.config.ts
import { sepal } from 'sepal/vite';
export default { plugins: [sepal()] };
```

```ts
// sepal.config.ts
import { defineConfig } from 'sepal/vite';
export default defineConfig({ app: { name: 'My App' } });
```

The `sepal` command runs, bundles and screenshots a built app:

```
sepal run [dist]                          open the built app in a window
sepal bundle [dist] [out-dir] [--sign <identity>] [--notarize <keychain-profile>]
             [--update-key <private-key-file>] [--update-notes <text>]
                                          build a macOS .app; with a key, also the signed update
sepal signer generate <private-key-file>  make the key pair updates are signed with
sepal screenshot <out.png> [dist] [WIDTHxHEIGHT]
```

Building is Vite's job: `vite build`, and `vite build --watch` for dev.
`fetch`, `console` and timers are globals. Everything else is in the
[documentation](https://github.com/etherCorps/sepal#readme).

MIT
