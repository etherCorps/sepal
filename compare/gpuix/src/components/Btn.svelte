<!--
	shadcn-svelte's Button as the Tracker uses it (variants default, outline, ghost, destructive;
	sizes default, sm, xs, icon-sm), drawn with this renderer's CSS. `style` stands in for the
	Tailwind classes the Tracker adds at each use (h-7, size-7, w-full, ...).
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		children,
		variant = 'default',
		size = 'default',
		disabled = false,
		style = '',
		testid,
		onclick
	}: {
		children: Snippet;
		variant?: 'default' | 'outline' | 'ghost' | 'destructive';
		size?: 'default' | 'sm' | 'xs' | 'icon-sm';
		disabled?: boolean;
		style?: string;
		testid?: string;
		onclick?: () => void;
	} = $props();
</script>

<button class="btn {variant} s-{size}" class:disabled {disabled} {style} {testid} hitbox="self" onclick={() => onclick?.()}>
	{@render children()}
</button>

<style>
	.btn {
		display: flex;
		flex-direction: row;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: transparent;
		font-size: 14px;
		font-weight: 500;
		white-space: nowrap;
		cursor: pointer;
		user-select: none;
	}
	.default { background-color: var(--primary); color: var(--primary-foreground); }
	.default:hover { background-color: var(--primary-80); }
	.outline { border-color: var(--border); background-color: var(--background); color: var(--foreground); }
	.outline:hover { background-color: var(--muted); }
	.ghost { color: var(--foreground); }
	.ghost:hover { background-color: var(--muted); }
	.destructive { background-color: var(--destructive-10); color: var(--destructive); }
	.destructive:hover { background-color: var(--destructive-20); }
	.s-default { height: 36px; gap: 6px; padding: 0 10px; }
	.s-sm { height: 32px; gap: 4px; padding: 0 10px; }
	.s-xs { height: 24px; gap: 4px; padding: 0 8px; font-size: 12px; }
	.s-icon-sm { width: 32px; height: 32px; }
	.disabled { opacity: 0.5; pointer-events: none; }
</style>
