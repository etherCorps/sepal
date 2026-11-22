<!--
	The root layout: it stays mounted while the pages inside it change, so
	`visits` below counts navigations instead of resetting.
-->
<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { back, forward, goto } from '$app/navigation';
	import { page } from '$app/state';

	let { data, children }: { data: { app: string }; children: Snippet } = $props();

	let clicks = $state(0);
</script>

<div class="flex h-screen flex-col bg-slate-50 font-sans text-slate-900">
	<header class="flex items-center gap-4 border-b border-slate-200 bg-white px-6 py-3">
		<strong id="app-name">{data.app}</strong>
		<nav class="flex gap-3 text-indigo-600">
			<a id="nav-home" href="/">Home</a>
			<a id="nav-items" href="/items">Items</a>
			<a id="nav-about" href="/about">About</a>
			<a id="nav-settings" href="/settings">Settings</a>
			<a id="nav-docs" href="/docs/guide/install">Docs</a>
			<a id="nav-old" href="/old">Old</a>
			<a id="nav-slow" href="/slow">Slow</a>
			<a id="nav-missing" href="/nowhere">Missing</a>
		</nav>
	</header>

	<div class="flex items-center gap-3 px-6 py-2 text-sm text-slate-600">
		<button id="back" class="rounded border border-slate-300 px-2 py-1" onclick={() => back()}>back</button>
		<button id="forward" class="rounded border border-slate-300 px-2 py-1" onclick={() => forward()}>forward</button>
		<button id="goto" class="rounded border border-slate-300 px-2 py-1" onclick={() => goto('/items/3?from=button')}>
			goto item 3
		</button>
		<button id="layout-clicks" class="rounded border border-slate-300 px-2 py-1" onclick={() => clicks++}>
			layout clicks: {clicks}
		</button>
		<span id="location">{page.url.pathname}</span>
		<span id="route-id">{page.route.id}</span>
	</div>

	<main class="min-h-0 flex-1 px-6 py-4">
		{@render children()}
	</main>
</div>
