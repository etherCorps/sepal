<!--
	One board card (examples/tracker/src/lib/board/Card.svelte): key, avatar, title (two lines),
	priority glyph, up to two label dots, estimate. `onnode` hands the board the card's element,
	which the status menu opens under (the Tracker's `customAnchor`).
-->
<script lang="ts">
	import type { ShadowNode } from 'gpuix-svelte';
	import { navigate } from 'gpuix-svelte-ui';
	import { PRIORITY_SVG, avatar } from '../glyphs.ts';
	import { label_by_id, type Issue } from '../seed.ts';

	let { issue, focused, onnode }: { issue: Issue; focused: boolean; onnode: (key: string, node: ShadowNode | null) => void } = $props();

	const priority = $derived(PRIORITY_SVG[issue.priority]);
	const who = $derived(avatar(issue.assignee));
</script>

<div
	class="card"
	class:focused
	testid="card"
	data-key={issue.key}
	hitbox="self"
	onclick={() => navigate(`/issues/${issue.key}`)}
	{@attach (node: ShadowNode) => {
		const key = issue.key;
		onnode(key, node);
		return () => onnode(key, null);
	}}
><div class="top"><text class="key">{issue.key}</text><div class="avatar" class:empty={who.empty} style={who.style}>{who.initials}</div></div><text class="title">{issue.title}</text><div class="bottom"><svg class="glyph {priority.tone}" source={priority.source}></svg>{#each issue.labelIds.slice(0, 2) as label (label)}<div class="dot" style="background-color: var(--hue-{label_by_id[label]?.hue ?? 1})"></div>{/each}<text class="estimate">{issue.estimate ?? ''}</text></div></div>

<style>
	.card {
		display: flex;
		flex-direction: column;
		flex-shrink: 0;
		gap: 6px;
		padding: 10px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--card);
		cursor: pointer;
	}
	.card:hover { border-color: var(--input); }
	.card.focused { outline-width: 2px; outline-color: var(--ring); }
	.top { display: flex; flex-direction: row; align-items: center; justify-content: space-between; height: 20px; }
	.key { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.avatar { display: flex; align-items: center; justify-content: center; flex-shrink: 0; border-radius: 999px; font-weight: 600; line-height: 10px; }
	.avatar.empty { border-width: 1px; border-style: dashed; border-color: var(--muted-foreground-60); }
	.title { line-clamp: 2; overflow: hidden; font-size: 13px; line-height: 18.85px; font-weight: 500; color: var(--foreground); }
	.bottom { display: flex; flex-direction: row; align-items: center; gap: 6px; height: 16px; }
	.glyph { width: 14px; height: 14px; flex-shrink: 0; }
	.muted-foreground { color: var(--muted-foreground); }
	.ink-2 { color: var(--ink-2); }
	.priority-urgent { color: var(--priority-urgent); }
	.dot { width: 6px; height: 6px; flex-shrink: 0; border-radius: 999px; }
	.estimate { flex-grow: 1; text-align: right; font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
</style>
