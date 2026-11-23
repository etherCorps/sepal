<!--
	The Tracker's Pick (a Select over a fixed list of ids) on gpuix-svelte-ui's Select. Controlled:
	it shows `value` and reports a choice through `onpick`. gpuix-svelte-ui's Select takes text
	options only, so the glyphs the Tracker draws in the trigger and the items are not here; the
	mono prefix ("Group", "Sort") and the "filter set" dot sit beside the trigger instead of inside.
	Variants become the Select's colour tokens, handed in as style props.
-->
<script lang="ts">
	import { Select } from 'gpuix-svelte-ui';
	import type { Item } from '../edit.ts';

	let {
		value,
		items,
		onpick,
		placeholder = '',
		prefix = '',
		dot = false,
		variant = 'outline',
		testid
	}: {
		value: string;
		items: Item[];
		onpick: (value: string) => void;
		placeholder?: string;
		prefix?: string;
		dot?: boolean;
		variant?: 'outline' | 'ghost' | 'graphite';
		testid?: string;
	} = $props();

	const chosen = $derived(items.some((item) => item.value === value) ? value : null);
	const choose = (next: string) => {
		if (next && next !== value) onpick(next);
	};
</script>

<div class="pick">
	{#if prefix}<text class="prefix" class:dark={variant === 'graphite'}>{prefix}</text>{/if}
	{#if variant === 'ghost'}
		<Select options={items} value={chosen} onchange={choose} {placeholder} testid={testid ?? null} --ui-border-strong="transparent" --ui-panel="transparent" />
	{:else if variant === 'graphite'}
		<Select
			options={items}
			value={chosen}
			onchange={choose}
			{placeholder}
			testid={testid ?? null}
			--ui-border-strong="transparent"
			--ui-panel="transparent"
			--ui-ink="var(--graphite-foreground)"
			--ui-muted="var(--graphite-muted)"
			--ui-raised-hover="var(--graphite-2)"
			--popover-surface="var(--graphite)"
			--popover-border="var(--graphite-2)"
		/>
	{:else}
		<Select options={items} value={chosen} onchange={choose} {placeholder} testid={testid ?? null} />
	{/if}
	{#if dot}<div class="dot"></div>{/if}
</div>

<style>
	.pick { display: flex; flex-direction: row; align-items: center; flex-shrink: 0; gap: 6px; font-size: 13px; }
	.prefix { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.prefix.dark { color: var(--graphite-muted); }
	.dot { width: 6px; height: 6px; flex-shrink: 0; border-radius: 999px; background-color: var(--primary); }
</style>
