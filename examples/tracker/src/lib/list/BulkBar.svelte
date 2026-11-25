<!--
	The dark bar over the list while rows are checked. Placed with `left`, not
	a translate, so tests can click it where its box says.
-->
<script lang="ts">
	import XIcon from '@lucide/svelte/icons/x';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '#lib/components/ui/dialog/index.ts';
	import { count, plural } from '#lib/format.ts';
	import type { StatusId } from '#lib/seed.ts';
	import { checked, ui } from '#lib/ui.svelte.ts';
	import Pick from './Pick.svelte';
	import { edit, people_items, remove, status_items } from './edit';

	let confirming = $state(false);

	$effect(() => {
		ui.bulk_bar = true;
		return () => (ui.bulk_bar = false);
	});

	const assignees = $derived([{ value: 'none', label: 'Unassigned' }, ...people_items()]);
	const n = $derived(checked.size);

	function confirm_delete() {
		remove([...checked]);
		checked.clear();
		confirming = false;
	}
</script>

<!-- Centred in the list. The toasts an Undo after every action leaves stack above it (Toasts.svelte). -->
<div
	data-testid="bulk-bar"
	style="left: calc(50% - 280px);"
	class="bg-graphite text-graphite-foreground absolute bottom-4 z-10 flex h-11 w-[560px] items-center gap-1 rounded-lg py-1.5 pr-1.5 pl-3.5 shadow-sm"
>
	<span data-testid="bulk-count" class="mr-auto font-mono text-xs">{count(n)} selected</span>
	<Pick
		testid="bulk-status"
		variant="graphite"
		kind="status"
		value=""
		placeholder="Set status"
		items={status_items}
		onpick={(status) => edit([...checked], { status: status as StatusId })}
	/>
	<Pick
		testid="bulk-assign"
		variant="graphite"
		kind="assignee"
		value=""
		placeholder="Assign"
		items={assignees}
		onpick={(id) => edit([...checked], { assignee: id === 'none' ? null : id })}
	/>
	<Button
		data-testid="bulk-delete"
		variant="ghost"
		size="sm"
		class="text-destructive hover:bg-graphite-2 hover:text-destructive h-7"
		onclick={() => (confirming = true)}>Delete</Button
	>
	<Button
		data-testid="bulk-clear"
		variant="ghost"
		size="icon-sm"
		class="text-graphite-muted hover:bg-graphite-2 hover:text-graphite-foreground size-7"
		aria-label="Clear selection"
		onclick={() => checked.clear()}><XIcon class="size-4" strokeWidth={1.75} /></Button
	>
</div>

<Dialog bind:open={confirming}>
	<DialogContent data-testid="confirm-delete" class="sm:max-w-[440px]" showCloseButton={false}>
		<DialogHeader>
			<DialogTitle class="font-display text-base font-medium">Delete {plural(n, 'issue')}?</DialogTitle>
			<DialogDescription>They leave the list and the board. Undo brings them back.</DialogDescription>
		</DialogHeader>
		<DialogFooter>
			<Button variant="ghost" onclick={() => (confirming = false)}>Cancel</Button>
			<Button data-testid="confirm-delete-submit" variant="destructive" onclick={confirm_delete}>Delete {plural(n, 'issue')}</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>
