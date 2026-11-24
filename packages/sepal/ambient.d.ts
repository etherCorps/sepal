// Module names the Vite preset provides. Add
// "sepal/ambient" to `types` in tsconfig.json.

declare module '$app/navigation' {
	export * from 'sepal/navigation';
}

declare module '$app/state' {
	export * from 'sepal/state';
}

// The route table generated from the routes directory. The built-in router
// runs on it; a router of your own can import it too.
declare module 'virtual:sepal/routes' {
	import type { Route, RouteNode } from 'sepal/router';
	import type { Component } from 'svelte';

	export const routes: Route[];
	export const fallback: { layouts: RouteNode[]; error: Component<any> | null };
}
