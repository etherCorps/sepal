<!--
	A Select over a fixed list of ids, drawn with the id's glyph (status,
	priority, person, project). Controlled: it shows `value` and reports a
	choice through `onpick`; the caller decides what changes. The toolbar,
	the bulk bar and the detail pane all use it.
-->
<script lang="ts">
	import * as Select from '$lib/components/ui/select/index.js';
	import { Avatar, PriorityIcon, ProjectSquare, StatusIcon } from '$lib/glyphs';
	import type { PriorityId, StatusId } from '$lib/seed';
	import { cn } from '$lib/utils';
	import type { Item } from './edit';

	type Kind = 'status' | 'priority' | 'assignee' | 'project' | 'plain';

	let {
		value,
		items,
		onpick,
		kind = 'plain',
		placeholder = '',
		prefix = '',
		dot = false,
		variant = 'outline',
		testid,
		class: className = ''
	}: {
		value: string;
		items: Item[];
		onpick: (value: string) => void;
		kind?: Kind;
		/** shown when `value` is none of the items */
		placeholder?: string;
		/** a mono label before the value: "Group", "Sort" */
		prefix?: string;
		/** a cobalt dot: the filter is set */
		dot?: boolean;
		variant?: 'outline' | 'ghost' | 'graphite';
		testid?: string;
		class?: string;
	} = $props();

	const current = $derived(items.find((item) => item.value === value));

	const variants = {
		outline: 'border-border bg-background hover:bg-accent text-foreground',
		ghost: 'hover:bg-accent text-foreground border-transparent shadow-none',
		graphite: 'text-graphite-foreground data-placeholder:text-graphite-foreground hover:bg-graphite-2 border-transparent shadow-none [&_svg.lucide]:text-graphite-muted'
	};
</script>

{#snippet glyph(id: string)}
	{#if kind === 'status'}
		<StatusIcon status={id as StatusId} />
	{:else if kind === 'priority'}
		<PriorityIcon priority={id as PriorityId} />
	{:else if kind === 'assignee'}
		<Avatar id={id === 'none' ? null : id} />
	{:else if kind === 'project'}
		<ProjectSquare project={id} />
	{/if}
{/snippet}

<Select.Root
	type="single"
	bind:value={
		() => value,
		(next) => {
			if (next && next !== value) onpick(next);
		}
	}
>
	<Select.Trigger
		size="sm"
		data-testid={testid}
		class={cn('data-[size=sm]:h-7 gap-1.5 px-2 text-[13px]', variants[variant], className)}
	>
		{#if prefix}<span class={cn('font-mono text-[11px]', variant === 'graphite' ? 'text-graphite-muted' : 'text-muted-foreground')}>{prefix}</span>{/if}
		{#if current && kind !== 'plain'}{@render glyph(current.value)}{/if}
		<span class="truncate">{current?.label ?? placeholder}</span>
		{#if dot}<span class="bg-primary size-1.5 shrink-0 rounded-full"></span>{/if}
	</Select.Trigger>
	<!--
		People come in two columns: a list taller than its box would scroll, and
		Blitz hit-tests the clipped items too, so one that falls over the
		trigger gets picked by the click that opened the list.
	-->
	<Select.Content
		class={kind === 'assignee'
			? 'max-h-none [&_[data-select-viewport]]:grid [&_[data-select-viewport]]:grid-cols-2'
			: 'max-h-80 overflow-y-scroll'}
	>
		{#each items as item (item.value)}
			<Select.Item value={item.value} label={item.label} class="text-[13px]">
				{#if kind !== 'plain' && item.value !== 'all'}{@render glyph(item.value)}{/if}
				{item.label}
			</Select.Item>
		{/each}
	</Select.Content>
</Select.Root>
