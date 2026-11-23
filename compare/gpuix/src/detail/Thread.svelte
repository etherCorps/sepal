<!--
	Comments and activity for one issue, and the box to comment in
	(examples/tracker/src/lib/detail/Thread.svelte). gpuix-svelte-ui's Tabs take text labels, so the
	counts are part of the label rather than a muted mono span beside it.
-->
<script lang="ts">
	import { blur, type GpuixEvent } from 'gpuix-svelte';
	import { Tabs } from 'gpuix-svelte-ui';
	import Avatar from '../components/Avatar.svelte';
	import Btn from '../components/Btn.svelte';
	import StatusIcon from '../components/StatusIcon.svelte';
	import { long_date, short_date } from '../format.ts';
	import { add_comment, person, thread, undo_action } from '../store.svelte.ts';
	import { push } from '../ui.svelte.ts';

	let { key }: { key: string } = $props();

	const details = $derived(thread(key));
	let tab = $state('comments');
	let draft = $state('');

	function post() {
		if (!draft.trim()) return;
		if (add_comment(key, draft)) {
			draft = '';
			tab = 'comments';
			push({ text: `Commented on ${key}`, action: undo_action() });
		}
	}
</script>

<div class="thread">
	<Tabs
		testid="tab"
		items={[
			{ value: 'comments', label: `Comments ${details.comments.length}` },
			{ value: 'activity', label: `Activity ${details.activity.length}` }
		]}
		value={tab}
		onchange={(v) => (tab = v)}
	/>
	{#if tab === 'comments'}
		<div class="comments">
			{#each details.comments as comment, i (i)}
				<div class="comment" testid="comment">
					<Avatar id={comment.author} />
					<div class="comment-body">
						<div class="comment-head">
							<text class="author">{person(comment.author)?.name}</text>
							<text class="when">{long_date(comment.at)}</text>
						</div>
						<text class="text">{comment.body}</text>
					</div>
				</div>
			{:else}
				<text class="none">No comments yet.</text>
			{/each}
		</div>
	{:else}
		<div class="activity">
			{#each details.activity as entry, i (i)}
				<div class="entry" testid="activity">
					{#if entry.status}
						<StatusIcon status={entry.status} size={12} />
					{:else}
						<div class="bullet"></div>
					{/if}
					<text class="who">{person(entry.author)?.name}</text><text class="entry-text">{entry.text} · {short_date(entry.at)}</text>
				</div>
			{/each}
		</div>
	{/if}
</div>

<div class="compose">
	<input
		class="field"
		testid="comment-input"
		placeholder="Leave a comment…"
		value={draft}
		onchange={(e) => (draft = e.value)}
		onsubmit={post}
		onkeydown={(e: GpuixEvent) => {
			// Escape leaves the field and keeps the draft, rather than closing the pane.
			if (e.key === 'escape') {
				e.preventDefault();
				blur();
			}
		}}
	/>
	<Btn testid="comment-submit" size="sm" variant="outline" disabled={!draft.trim()} onclick={post}>Comment</Btn>
</div>

<style>
	.thread { display: flex; flex-direction: column; gap: 16px; }
	.comments { display: flex; flex-direction: column; gap: 16px; }
	.comment { display: flex; flex-direction: row; gap: 10px; }
	.comment-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
	.comment-head { display: flex; flex-direction: row; align-items: baseline; gap: 8px; }
	.author { font-size: 13px; font-weight: 500; color: var(--foreground); }
	.when { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.text { font-size: 13px; color: var(--ink-2); }
	.none { font-size: 12px; color: var(--muted-foreground); }
	.activity { display: flex; flex-direction: column; gap: 8px; }
	.entry { display: flex; flex-direction: row; align-items: center; gap: 8px; min-width: 0; font-size: 12px; color: var(--muted-foreground); }
	.bullet { width: 6px; height: 6px; margin: 0 3px; flex-shrink: 0; border-radius: 999px; background-color: var(--muted-foreground-50); }
	.entry-text { min-width: 0; flex-shrink: 1; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
	.who { flex-shrink: 0; margin-right: -4px; font-weight: 500; color: var(--ink-2); }
	.compose { display: flex; flex-direction: row; align-items: center; gap: 8px; padding-top: 8px; }
	.field {
		flex-grow: 1;
		flex-basis: 0;
		min-width: 0;
		height: 32px;
		padding: 0 10px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--input);
		font-size: 14px;
		color: var(--foreground);
	}
</style>
