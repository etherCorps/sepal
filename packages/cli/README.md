# @ethercorps/svelte-blitz-cli

The `svelte-blitz` command for [svelte-blitz](https://github.com/etherCorps/svelte-blitz) apps.

```
svelte-blitz run [dist]                          open the built app in a window
svelte-blitz bundle [dist] [out-dir] [--sign <identity>] [--notarize <keychain-profile>]
                    [--update-key <private-key-file>] [--update-notes <text>]
                                                 build a macOS .app; with a key, also the signed update
svelte-blitz signer generate <private-key-file>  make the key pair updates are signed with
svelte-blitz screenshot <out.png> [dist] [WIDTHxHEIGHT]
```

Building is Vite's job: `vite build`, and `vite build --watch` for dev.
[Documentation](https://github.com/etherCorps/svelte-blitz#readme).

MIT
