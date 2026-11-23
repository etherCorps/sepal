<!--
	/board (examples/tracker/src/routes/board/+page.svelte): one column per status, at most
	BOARD_LIMIT cards each, with the list's filters, and the same keys.
	`s` opens the status list under the focused card: gpuix-svelte-ui's Menu, opened at the card's
	painted box (the Tracker opens a hidden Select anchored to the card; this Select cannot be
	opened from code or anchored elsewhere).
-->
<script lang="ts">
	import { get_native, type ShadowNode } from 'gpuix-svelte';
	import { Menu, navigate } from 'gpuix-svelte-ui';
	import PageHeader from '../components/PageHeader.svelte';
	import StatusIcon from '../components/StatusIcon.svelte';
	import { count } from '../format.ts';
	import Filters from '../list/Filters.svelte';
	import { STATUSES, status_by_id, type StatusId } from '../seed.ts';
	import { data, set_scope, undo_action, update, view } from '../store.svelte.ts';
	import { push, use_keys } from '../ui.svelte.ts';
	import Card from './Card.svelte';

	$effect.pre(() => {
		set_scope({});
		view.week = null;
	});

	const shown = $derived(data.filtered.length);
	const total = $derived(data.total);

	// The scroller's height, handed down: a percentage height under a scroll container resolves to
	// the content's here, so the columns (the Tracker's h-full) would collapse to their headers.
	let height = $state(0);

	let focused = $state<string | null>(null);
	let picking = $state(false);
	let at_point = $state<{ x: number; y: number } | null>(null);
	const nodes = new Map<string, ShadowNode>();
	const onnode = (key: string, node: ShadowNode | null) => (node ? nodes.set(key, node) : nodes.delete(key));

	function pick() {
		const id = focused === null ? null : nodes.get(focused)?.nativeId;
		const box = id == null ? null : get_native()?.getElementBounds(id);
		if (!box) return;
		at_point = { x: box.x, y: box.y + box.height + 4 };
		picking = true;
	}

	const at = $derived.by(() => {
		if (focused === null) return null;
		const columns = data.board;
		for (let col = 0; col < columns.length; col++) {
			const row = columns[col].issues.findIndex((issue) => issue.key === focused);
			if (row !== -1) return { col, row, issue: columns[col].issues[row] };
		}
		return null;
	});

	function focus_first() {
		const column = data.board.find((c) => c.issues.length > 0);
		focused = column ? column.issues[0].key : null;
	}

	function across(step: number) {
		if (!at) return focus_first();
		const columns = data.board;
		for (let col = at.col + step; col >= 0 && col < columns.length; col += step) {
			const { issues } = columns[col];
			if (issues.length > 0) {
				focused = issues[Math.min(at.row, issues.length - 1)].key;
				return;
			}
		}
	}

	function down(step: number) {
		if (!at) return focus_first();
		const next = data.board[at.col].issues[at.row + step];
		if (next) focused = next.key;
	}

	function set_status(status: StatusId) {
		if (!at || at.issue.status === status) return;
		const key = at.issue.key;
		update(key, { status });
		push({ text: `Moved ${key} to ${status_by_id[status].name}`, status, action: undo_action() });
	}

	function shift(step: number) {
		if (!at) return;
		const next = STATUSES[at.col + step];
		if (next) set_status(next.id);
	}

	const when = (run: () => void) => () => (picking ? false : run());
	use_keys({
		h: when(() => across(-1)),
		l: when(() => across(1)),
		j: when(() => down(1)),
		k: when(() => down(-1)),
		ArrowLeft: when(() => across(-1)),
		ArrowRight: when(() => across(1)),
		ArrowDown: when(() => down(1)),
		ArrowUp: when(() => down(-1)),
		'Shift+h': when(() => shift(-1)),
		'Shift+l': when(() => shift(1)),
		s: when(() => {
			if (at) pick();
		}),
		Enter: when(() => {
			if (at) void navigate(`/issues/${at.issue.key}`);
		}),
		o: when(() => {
			if (at) void navigate(`/issues/${at.issue.key}`);
		}),
		Escape: () => {
			if (focused === null) return false;
			focused = null;
		}
	});
</script>

<PageHeader title="Board" count={shown === total ? count(total) : `${count(shown)} of ${count(total)}`} />
<div class="toolbar"><Filters /></div>

<div class="scroll" onlayout={(e) => (height = e.height ?? height)}>
	<div class="columns" style="height: {height}px">
		{#each data.board as column (column.status.id)}
			<div class="column" testid="board-column">
				<div class="head">
					<StatusIcon status={column.status.id} />
					<text class="name">{column.status.name}</text>
					<text class="count" testid="column-count">{count(column.total)}</text>
				</div>
				<div class="cards">
					{#each column.issues as issue (issue.key)}
						<Card {issue} focused={issue.key === focused} {onnode} />
					{/each}
					{#if column.total > column.issues.length}
						<button
							class="more"
							testid="column-more"
							hitbox="self"
							onclick={() => {
								view.status = column.status.id;
								void navigate('/issues');
							}}>+ {count(column.total - column.issues.length)} more in list</button
						>
					{/if}
				</div>
			</div>
		{/each}
	</div>
</div>

{#if at}
	<Menu
		open={picking}
		onclose={() => (picking = false)}
		position={at_point}
		testid="card-status"
		items={STATUSES.map((status) => ({ label: status.name, onselect: () => set_status(status.id) }))}
	>
		<div></div>
	</Menu>
{/if}

<style>
	.toolbar {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 8px;
		height: 44px;
		padding: 0 16px;
		overflow: hidden;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--border);
	}
	.scroll { display: flex; flex-direction: row; align-items: stretch; flex-grow: 1; flex-basis: 0; min-height: 0; overflow-x: scroll; scrollbar-color: var(--input); }
	.columns { display: flex; flex-direction: row; flex-shrink: 0; gap: 12px; padding: 16px; }
	.column { display: flex; flex-direction: column; width: 280px; flex-shrink: 0; min-height: 0; border-radius: 8px; background-color: var(--muted); }
	.head { display: flex; flex-direction: row; align-items: center; flex-shrink: 0; gap: 8px; height: 32px; padding: 0 12px; }
	.name { font-size: 12px; font-weight: 500; white-space: nowrap; color: var(--foreground); }
	.count { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.cards {
		display: flex;
		flex-direction: column;
		flex-grow: 1;
		flex-basis: 0;
		min-height: 0;
		gap: 6px;
		padding: 2px 6px 6px 6px;
		overflow-y: scroll;
		pointer-events: auto;
		scrollbar-color: var(--input);
	}
	.more { flex-shrink: 0; padding: 6px 4px; font-size: 12px; color: var(--muted-foreground); cursor: pointer; }
	.more:hover { color: var(--foreground); }
</style>
