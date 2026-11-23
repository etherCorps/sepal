<!-- Initials in a circle tinted with the person's hue. `id` null is unassigned: a dashed ring. -->
<script module lang="ts">
	import { person } from '$lib/store.svelte';

	/** The avatar as one span's attributes: rows and cards write the span inline. Reactive (the user's name). */
	export function avatar(id: string | null, size: 20 | 24 = 20) {
		const who = person(id);
		const box = `width: ${size}px; height: ${size}px;`;
		if (!who) return { class: 'border-muted-foreground/60 inline-block shrink-0 rounded-full border border-dashed', style: box, title: 'Unassigned', initials: '' };
		return {
			class: 'inline-flex shrink-0 items-center justify-center rounded-full font-semibold leading-none',
			style: `${box} font-size: ${size === 24 ? 11 : 10}px; background: color-mix(in oklch, var(--hue-${who.hue}) 18%, var(--background)); color: color-mix(in oklch, var(--hue-${who.hue}) 70%, black);`,
			title: who.name,
			initials: who.initials
		};
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils';

	let { id, size = 20, class: className = '' }: { id: string | null; size?: 20 | 24; class?: string } = $props();

	const a = $derived(avatar(id, size));
</script>

<span class={cn(a.class, className)} style={a.style} title={a.title}>{a.initials}</span>
