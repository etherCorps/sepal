# @ethercorps/calyx

Calyx, the host of [Sepal](https://github.com/etherCorps/sepal): the native program that opens the window,
runs the app's JavaScript and draws it.

Each platform's binary is a package of its own, which this one lists as
optional dependencies; a package manager installs only the one that fits the
machine:

| Package | Machine |
|---|---|
| `@ethercorps/calyx-darwin-arm64` | macOS, Apple silicon |
| `@ethercorps/calyx-darwin-x64` | macOS, Intel |
| `@ethercorps/calyx-linux-x64-gnu` | Linux x64, glibc 2.35 or newer |
| `@ethercorps/calyx-linux-arm64-gnu` | Linux arm64, glibc 2.35 or newer |

Each holds `bin/calyx`, its `LICENSE`, and `THIRD-PARTY-LICENSES.txt` for
the software compiled into the binary. All are pinned to the same exact
version as `sepal`.

You do not install this yourself: the `sepal` package depends on it.
Set `SEPAL_HOST` to use a calyx you built from source instead.

```js
import { find_host } from '@ethercorps/calyx';
find_host(); // { path, version }; throws, naming the package this machine needs, when there is none
```

MIT
