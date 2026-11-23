<!--
	The detail pane (examples/tracker/src/routes/issues/[key]/+page.svelte), at the right of the
	list. Mounted by IssuesLayout for `/issues/:key` and kept mounted while the key changes, as the
	Tracker's router keeps the page component. A key that never existed shows what the Tracker's
	issues/+error.svelte shows (the same two lines as an issue deleted while open).
	Copy link: this runtime has no clipboard API, so it takes the Tracker's failure path (a toast).
-->
<script lang="ts">
	import { blur, focus_element, type GpuixEvent, type ShadowNode } from 'gpuix-svelte';
	import { navigate } from 'gpuix-svelte-ui';
	import Btn from '../components/Btn.svelte';
	import Icon from '../components/Icon.svelte';
	import { edit, list_href } from '../edit.ts';
	import { get_issue, neighbor, thread } from '../store.svelte.ts';
	import { push, ui } from '../ui.svelte.ts';
	import Properties from './Properties.svelte';
	import Thread from './Thread.svelte';

	let { key }: { key: string } = $props();

	const issue = $derived(get_issue(key));
	const description = $derived(thread(key).description);
	const prev = $derived(neighbor(key, -1));
	const next = $derived(neighbor(key, 1));

	$effect.pre(() => {
		ui.highlighted = key;
	});

	// --- the title, edited in place -------------------------------------------

	let editing = $state(false);
	let draft = $state('');
	let error = $state('');
	let input = $state<ShadowNode | null>(null);

	function check(text: string): string {
		const length = text.trim().length;
		if (length === 0) return "Title can't be empty. Type at least 3 characters.";
		if (length < 3) return 'Type at least 3 characters.';
		if (length > 200) return 'Keep the title under 200 characters.';
		return '';
	}

	function start() {
		if (!issue) return;
		draft = issue.title;
		error = '';
		editing = true;
	}
	function save() {
		if (!issue || !editing) return;
		error = check(draft);
		if (error) return;
		editing = false;
		if (draft.trim() !== issue.title) edit([issue.key], { title: draft.trim() });
	}
	function cancel() {
		editing = false;
		error = '';
	}

	// Another issue (j/k) closes the editor without saving.
	$effect.pre(() => {
		key;
		cancel();
	});
	$effect(() => {
		if (editing && input) focus_element(input);
	});

	function title_keys(event: GpuixEvent) {
		if (event.key === 'escape') {
			// Before the shell's Escape, which would close the pane.
			event.preventDefault();
			cancel();
			blur();
		}
	}

	function copy_link() {
		if (!issue) return;
		push({ text: `Could not copy the link. Copy the key ${issue.key} instead.`, tone: 'error' });
	}
</script>

{#if issue}
	<div class="bar">
		<text class="key" testid="detail-key">{issue.key}</text>
		<Btn variant="ghost" size="icon-sm" style="width: 28px; height: 28px" onclick={copy_link}><Icon name="link" /></Btn>
		<div class="nav">
			<Btn testid="detail-prev" variant="ghost" size="icon-sm" style="width: 28px; height: 28px" disabled={prev === null} onclick={() => prev && navigate(`/issues/${prev}`)}>
				<Icon name="arrow-up" />
			</Btn>
			<Btn testid="detail-next" variant="ghost" size="icon-sm" style="width: 28px; height: 28px" disabled={next === null} onclick={() => next && navigate(`/issues/${next}`)}>
				<Icon name="arrow-down" />
			</Btn>
			<Btn testid="detail-close" variant="ghost" size="icon-sm" style="width: 28px; height: 28px" onclick={() => navigate(list_href())}>
				<Icon name="x" />
			</Btn>
		</div>
	</div>

	<div class="body">
		<div class="heading">
			{#if editing}
				<input
					class="title-input"
					class:invalid={error !== ''}
					testid="title-input"
					value={draft}
					onchange={(e) => {
						draft = e.value;
						if (error) error = check(draft);
					}}
					onsubmit={save}
					onkeydown={title_keys}
					{@attach (node: ShadowNode) => {
						input = node;
						return () => (input = null);
					}}
				/>
				<!-- In the gap below, so clicking the title moves nothing. -->
				{#if error}
					<text class="hint error" testid="title-error">{error}</text>
				{:else}
					<text class="hint">Enter saves, Esc cancels.</text>
				{/if}
			{:else}
				<button class="title" testid="detail-title" hitbox="self" onclick={start}>{issue.title}</button>
			{/if}
		</div>

		{#if description}
			<text class="description" testid="detail-description">{description}</text>
		{/if}

		<Properties {issue} />

		<Thread key={issue.key} />
	</div>
{:else}
	<div class="missing">
		<text class="missing-title" testid="error-message">No issue {key}.</text>
		<div class="missing-line">
			<text>It may have been deleted.</text>
			<button class="link" hitbox="self" onclick={() => navigate('/issues')}>Back to issues</button>
		</div>
	</div>
{/if}

<style>
	.bar {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 4px;
		height: 48px;
		padding: 0 8px 0 20px;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--border);
	}
	.key { margin-right: 4px; font-family: JetBrains Mono; font-size: 12px; color: var(--muted-foreground); }
	.nav { display: flex; flex-direction: row; align-items: center; justify-content: flex-end; flex-grow: 1; gap: 2px; }
	.body { display: flex; flex-direction: column; gap: 24px; padding: 20px 20px 40px 20px; }
	.heading { position: relative; display: flex; flex-direction: column; }
	.title {
		margin: 0 -8px;
		padding: 4px 8px;
		border-radius: 6px;
		font-family: Space Grotesk Light;
		font-size: 22px;
		font-weight: 500;
		line-height: 27.5px;
		color: var(--foreground);
		cursor: pointer;
	}
	.title:hover { background-color: var(--accent); }
	.title-input {
		height: 36px;
		margin: 0 -8px;
		padding: 4px 7px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--input);
		font-family: Space Grotesk Light;
		font-size: 22px;
		font-weight: 500;
		color: var(--foreground);
	}
	.title-input.invalid { border-color: var(--destructive); }
	.hint { position: absolute; top: 40px; left: 0; font-size: 12px; color: var(--muted-foreground); }
	.hint.error { color: var(--destructive); }
	.description { font-size: 14px; line-height: 22.4px; color: var(--ink-2); }
	.missing { display: flex; flex-direction: column; gap: 4px; padding: 20px; }
	.missing-title { font-size: 16px; font-weight: 500; color: var(--foreground); }
	.missing-line { display: flex; flex-direction: row; gap: 4px; font-size: 13px; color: var(--muted-foreground); }
	.link { color: var(--primary); cursor: pointer; }
</style>
