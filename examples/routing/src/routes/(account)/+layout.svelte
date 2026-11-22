<!-- A group: this layout wraps /settings without adding anything to its path. -->
<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<section id="account" class="rounded-lg border border-slate-200 bg-white p-4">
	<p class="text-sm text-slate-500">Account</p>
	{@render children()}
</section>
