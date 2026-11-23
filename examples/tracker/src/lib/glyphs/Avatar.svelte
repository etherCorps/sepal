<!-- Initials in a circle tinted with the person's hue. `id` null is unassigned: a dashed ring. -->
<script lang="ts">
	import { cn } from '$lib/utils';
	import { person } from '$lib/store.svelte';

	let { id, size = 20, class: className = '' }: { id: string | null; size?: 20 | 24; class?: string } = $props();

	const who = $derived(person(id));
</script>

{#if who}
	<span
		class={cn('inline-flex shrink-0 items-center justify-center rounded-full font-semibold leading-none', className)}
		style="width: {size}px; height: {size}px; font-size: {size === 24 ? 11 : 10}px; background: color-mix(in oklch, var(--hue-{who.hue}) 18%, var(--background)); color: color-mix(in oklch, var(--hue-{who.hue}) 70%, black);"
		title={who.name}>{who.initials}</span
	>
{:else}
	<span
		class={cn('border-muted-foreground/60 inline-block shrink-0 rounded-full border border-dashed', className)}
		style="width: {size}px; height: {size}px;"
		title="Unassigned"
	></span>
{/if}
