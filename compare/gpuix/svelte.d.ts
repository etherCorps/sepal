declare module '*.svelte' {
	const component: import('gpuix-svelte').AnyComponent;
	export default component;
}

declare module '*.ttf' {
	const path: string;
	export default path;
}
