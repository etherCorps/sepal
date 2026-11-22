# @ethercorps/svelte-blitz-vite

The Vite preset for [svelte-blitz](https://github.com/etherCorps/svelte-blitz) apps: compiles Svelte for the Blitz
renderer, writes the one `app.js` and `app.css` the host loads, reads
`app.config.ts`, and in `vite build --watch` opens the app and reloads it on
save. It also holds the file-based router (`src/routes`, `$app/navigation`,
`$app/state`).

```ts
// vite.config.ts
import { svelteBlitz } from '@ethercorps/svelte-blitz-vite';
export default { plugins: [svelteBlitz()] };
```

```ts
// app.config.ts
import { defineConfig } from '@ethercorps/svelte-blitz-vite';
export default defineConfig({ app: { name: 'My App' } });
```

For a SvelteKit app, `svelteBlitzKit()` from
`@ethercorps/svelte-blitz-vite/sveltekit` takes the place of both
`sveltekit()` and the adapter.

The starter sets all of this up: `pnpm create @ethercorps/svelte-blitz my-app`.
[Documentation](https://github.com/etherCorps/svelte-blitz#readme).

MIT
