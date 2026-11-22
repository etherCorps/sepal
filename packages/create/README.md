# @ethercorps/create-svelte-blitz

Starts a [svelte-blitz](https://github.com/etherCorps/svelte-blitz) project: a Svelte app that runs in a native
window, without a webview.

```sh
pnpm create @ethercorps/svelte-blitz my-app
cd my-app
pnpm install
pnpm dev        # opens the app, reloads on save
pnpm bundle     # my-app.app
```

TypeScript, Tailwind, pages as files in `src/routes`, an icon and a
`static/` directory. You are asked which router to use; to say so ahead of
time, `--router built-in` (svelte-blitz's own, the default) or
`--router sveltekit` (SvelteKit as a single-page app).

[Documentation](https://github.com/etherCorps/svelte-blitz#readme).

MIT
