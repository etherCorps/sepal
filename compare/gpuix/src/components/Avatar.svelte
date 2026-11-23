<!-- Initials in a circle tinted with the person's hue. `id` null is unassigned: a dashed ring. -->
<script lang="ts">
	import { avatar } from '../glyphs.ts';

	let { id, size = 20 }: { id: string | null; size?: 20 | 24 } = $props();
	const a = $derived(avatar(id, size));
</script>

<div class="avatar" class:empty={a.empty} style={a.style}>{a.initials}</div>

<style>
	.avatar { display: flex; align-items: center; justify-content: center; flex-shrink: 0; border-radius: 999px; font-weight: 600; line-height: 10px; }
	.empty { border-width: 1px; border-style: dashed; border-color: var(--muted-foreground-60); }
</style>
