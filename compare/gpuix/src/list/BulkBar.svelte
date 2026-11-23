<!--
	The dark bar over the list while rows are checked (examples/tracker/src/lib/list/BulkBar.svelte).
	Centred by a full-width row, since `left` takes pixels only; no shadow (GPUI has no box-shadow).
-->
<script lang="ts">
	import { Dialog } from 'gpuix-svelte-ui';
	import Btn from '../components/Btn.svelte';
	import Icon from '../components/Icon.svelte';
	import Pick from '../components/Pick.svelte';
	import { edit, people_items, remove, status_items } from '../edit.ts';
	import { count, plural } from '../format.ts';
	import type { StatusId } from '../seed.ts';
	import { checked, ui, use_modal } from '../ui.svelte.ts';

	let confirming = $state(false);
	use_modal(() => confirming);

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

<div class="dock">
	<div class="bar" testid="bulk-bar">
		<text class="count" testid="bulk-count">{count(n)} selected</text>
		<Pick testid="bulk-status" variant="graphite" value="" placeholder="Set status" items={status_items} onpick={(status) => edit([...checked], { status: status as StatusId })} />
		<Pick testid="bulk-assign" variant="graphite" value="" placeholder="Assign" items={assignees} onpick={(id) => edit([...checked], { assignee: id === 'none' ? null : id })} />
		<Btn testid="bulk-delete" variant="ghost" size="sm" style="height: 28px; color: var(--destructive)" onclick={() => (confirming = true)}>Delete</Btn>
		<Btn testid="bulk-clear" variant="ghost" size="icon-sm" style="width: 28px; height: 28px" onclick={() => checked.clear()}>
			<Icon name="x" color="var(--graphite-muted)" />
		</Btn>
	</div>
</div>

<Dialog open={confirming} onclose={() => (confirming = false)} title="Delete {plural(n, 'issue')}?" description="They leave the list and the board. Undo brings them back." testid="confirm-delete">
	<div></div>
	{#snippet actions()}
		<Btn variant="ghost" onclick={() => (confirming = false)}>Cancel</Btn>
		<Btn testid="confirm-delete-submit" variant="destructive" onclick={confirm_delete}>Delete {plural(n, 'issue')}</Btn>
	{/snippet}
</Dialog>

<style>
	.dock { position: absolute; left: 0; right: 0; bottom: 16px; display: flex; flex-direction: row; justify-content: center; pointer-events: none; }
	.bar {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 4px;
		width: 560px;
		height: 44px;
		padding: 6px 6px 6px 14px;
		border-radius: 8px;
		background-color: var(--graphite);
		color: var(--graphite-foreground);
		pointer-events: auto;
	}
	.count { flex-grow: 1; font-family: JetBrains Mono; font-size: 12px; }
</style>
