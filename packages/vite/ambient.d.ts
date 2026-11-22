// Module names the Vite preset provides. Add
// "@ethercorps/svelte-blitz-vite/ambient" to `types` in tsconfig.json.

declare module '$app/navigation' {
	export * from '@ethercorps/svelte-blitz-vite/navigation';
}

declare module '$app/state' {
	export * from '@ethercorps/svelte-blitz-vite/state';
}

// The route table generated from the routes directory. The built-in router
// runs on it; a router of your own can import it too.
declare module 'virtual:svelte-blitz/routes' {
	import type { Route, RouteNode } from '@ethercorps/svelte-blitz-vite/router';
	import type { Component } from 'svelte';

	export const routes: Route[];
	export const fallback: { layouts: RouteNode[]; error: Component<any> | null };
}
