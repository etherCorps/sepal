# create-sepal

Starts a [Sepal](https://github.com/etherCorps/sepal) project: a Svelte app that runs in a native
window, without a webview.

```sh
pnpm create sepal my-app
cd my-app
pnpm install
pnpm dev        # opens the app, reloads on save
pnpm bundle     # my-app.app
```

TypeScript, Tailwind, pages as files in `src/routes`, an icon and a
`static/` directory. You are asked which router to use; to say so ahead of
time, `--router built-in` (Sepal's own, the default) or
`--router sveltekit` (SvelteKit as a single-page app).

[Documentation](https://github.com/etherCorps/sepal#readme).

MIT
