# @ethercorps/svelte-blitz-host

The host of [svelte-blitz](https://github.com/etherCorps/svelte-blitz): the native program that opens the window,
runs the app's JavaScript and draws it. This package holds the prebuilt binary
for each platform under `bin/<platform>/`, so that nobody who installs from
npm needs Rust, and the lookup that picks the one for this machine.

You do not install it yourself: the Vite preset and the CLI depend on it.
Set `SVELTE_BLITZ_HOST` to use a host you built from source instead.

```js
import { find_host } from '@ethercorps/svelte-blitz-host';
find_host(); // the binary's path, or null when there is none for this platform
```

The binaries contain third-party software; see `THIRD-PARTY-LICENSES.txt`.

MIT
