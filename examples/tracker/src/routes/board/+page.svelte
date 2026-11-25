<!--
	/board: one column per status, at most BOARD_LIMIT cards each, with the
	list's filters (docs/tracker-design.md, Board).

	Keys: h/l and j/k move the focused card, Shift+h/Shift+l move it to the
	next status, s picks its status, Enter or o opens it. The status Select
	exists once, for the focused card, and is anchored to it: a Select per card
	would put 300 Bits UI roots on the page.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '#lib/components/ui/select/index.ts';
	import Card from '#lib/board/Card.svelte';
	import Filters from '#lib/list/Filters.svelte';
	import { count } from '#lib/format.ts';
	import { StatusIcon } from '#lib/glyphs/index.ts';
	import { STATUSES, status_by_id, type StatusId } from '#lib/seed.ts';
	import { data, set_scope, undo_action, update, view } from '#lib/store.svelte.ts';
	import { push, use_keys } from '#lib/ui.svelte.ts';

	$effect.pre(() => {
		set_scope({});
		// The week from Insights belongs to the list; the board has no chip to clear it.
		view.week = null;
	});

	const shown = $derived(data.filtered.length);
	const total = $derived(data.total);

	/** The focused card's key. Survives the card moving to another column. */
	let focused = $state<string | null>(null);
	let picking = $state(false);
	let trigger = $state<HTMLElement | null>(null);

	// Bits UI reads arrows, Enter and typeahead from its trigger, so the hidden
	// trigger holds the focus while the list is open, and lets go after: a
	// focused trigger would take Enter and letters for itself.
	function pick() {
		trigger?.focus();
		picking = true;
	}
	function picked(open: boolean) {
		if (!open) queueMicrotask(() => trigger?.blur());
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

	// While the status list is open it owns the keys.
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
			if (at) void goto(`/issues/${at.issue.key}`);
		}),
		o: when(() => {
			if (at) void goto(`/issues/${at.issue.key}`);
		}),
		Escape: () => {
			if (focused === null) return false;
			focused = null;
		}
	});
</script>

<PageHeader title="Board" count={shown === total ? count(total) : `${count(shown)} of ${count(total)}`} />
<div class="flex h-11 shrink-0 items-center gap-2 border-b px-4"><Filters /></div>

<div class="min-h-0 flex-1 overflow-x-scroll">
	<div class="flex h-full w-max gap-3 p-4">
		{#each data.board as column (column.status.id)}
			<section data-testid="board-column" data-status={column.status.id} class="bg-muted flex min-h-0 w-[280px] shrink-0 flex-col rounded-lg">
				<header class="flex h-8 shrink-0 items-center gap-2 px-3">
					<StatusIcon status={column.status.id} />
					<span class="text-foreground text-xs font-medium whitespace-nowrap">{column.status.name}</span>
					<span data-testid="column-count" class="text-muted-foreground font-mono text-[11px]">{count(column.total)}</span>
				</header>
				<div class="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-scroll px-1.5 pt-0.5 pb-1.5">
					{#each column.issues as issue (issue.key)}
						<Card {issue} focused={issue.key === focused} />
					{/each}
					{#if column.total > column.issues.length}
						<a
							data-testid="column-more"
							href="/issues"
							class="text-muted-foreground hover:text-foreground shrink-0 px-1 py-1.5 text-xs"
							onclick={() => (view.status = column.status.id)}>+ {count(column.total - column.issues.length)} more in list</a
						>
					{/if}
				</div>
			</section>
		{/each}
	</div>
</div>

{#if at}
	<Select type="single" bind:open={picking} onOpenChange={picked} value={at.issue.status} onValueChange={(value) => set_status(value as StatusId)}>
		<!-- Opened by `s`, never clicked: the list is anchored to the card instead. -->
		<SelectTrigger bind:ref={trigger} data-testid="card-status-trigger" tabindex={-1} aria-hidden="true" class="pointer-events-none absolute size-0 overflow-hidden border-0 p-0 opacity-0" />
		<SelectContent
			data-testid="card-status"
			customAnchor={`[data-key="${at.issue.key}"]`}
			side="bottom"
			align="start"
		>
			{#each STATUSES as status (status.id)}
				<SelectItem value={status.id} label={status.name}><StatusIcon status={status.id} />{status.name}</SelectItem>
			{/each}
		</SelectContent>
	</Select>
{/if}
