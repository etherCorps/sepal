<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';

	let { children }: { children: Snippet } = $props();
</script>

<div class="bg-background text-foreground flex h-screen flex-col">
	<nav class="flex gap-4 border-b px-6 py-3">
		<a id="nav-a" href="/">A</a>
		<a id="nav-b" href="/b">B</a>
		<span id="location">{page.url.pathname}</span>
	</nav>
	<main class="p-6">{@render children()}</main>
</div>
