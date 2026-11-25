<!-- The root layout wraps every page. It stays mounted while pages change. -->
<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';

	let { children }: { children: Snippet } = $props();

	const links = [
		{ href: '/', label: 'Home' },
		{ href: '/about', label: 'About' }
	];
</script>

<div class="flex h-screen flex-col bg-slate-50 font-sans text-slate-900">
	<nav class="flex items-center gap-4 border-b border-slate-200 bg-white px-6 py-3">
		<!-- Files in static/ are copied next to the app, addressed from the root. -->
		<img src="/logo.svg" alt="" width="28" height="28" />
		{#each links as link (link.href)}
			<a
				href={link.href}
				class={page.url.pathname === link.href ? 'font-semibold text-indigo-600' : 'text-slate-600 hover:text-slate-900'}
			>
				{link.label}
			</a>
		{/each}
	</nav>

	<main class="flex min-h-0 flex-1 flex-col items-center justify-center gap-6">
		{@render children()}
	</main>
</div>
