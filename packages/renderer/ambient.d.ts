// The SvelteKit-style module names the Vite preset provides. Add
// "@ethercorps/svelte-blitz/ambient" to `types` in tsconfig.json.

declare module '$app/navigation' {
	export * from '@ethercorps/svelte-blitz/navigation';
}

declare module '$app/state' {
	export * from '@ethercorps/svelte-blitz/state';
}
