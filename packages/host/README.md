# @ethercorps/calyx

Calyx, the host of [Sepal](https://github.com/etherCorps/sepal): the native program that opens the window,
runs the app's JavaScript and draws it. This package holds the prebuilt binary
for each platform under `bin/<platform>/`, so that nobody who installs from
npm needs Rust, and the lookup that picks the one for this machine.

You do not install it yourself: the `sepal` package depends on it.
Set `SEPAL_HOST` to use a host you built from source instead.

```js
import { find_host } from '@ethercorps/calyx';
find_host(); // the binary's path, or null when there is none for this platform
```

The binaries contain third-party software; see `THIRD-PARTY-LICENSES.txt`.

MIT
