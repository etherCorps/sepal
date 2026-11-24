<!--
	The detail pane (docs/tracker-design.md, Detail pane), at the right of the
	list in issues/+layout.svelte. j/k, Escape and the highlight are the
	list's keys (IssueList.svelte); this page follows the route.
-->
<script lang="ts">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import LinkIcon from '@lucide/svelte/icons/link';
	import XIcon from '@lucide/svelte/icons/x';
	import { writeText } from 'sepal/clipboard';
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import Properties from '$lib/detail/Properties.svelte';
	import Thread from '$lib/detail/Thread.svelte';
	import { edit, list_href } from '$lib/list/edit';
	import { get_issue, neighbor, thread } from '$lib/store.svelte';
	import { push, ui } from '$lib/ui.svelte';

	let { data }: { data: { key: string } } = $props();

	const issue = $derived(get_issue(data.key));
	const description = $derived(thread(data.key).description);
	const prev = $derived(neighbor(data.key, -1));
	const next = $derived(neighbor(data.key, 1));

	$effect.pre(() => {
		ui.highlighted = data.key;
	});

	// --- the title, edited in place -------------------------------------------

	let editing = $state(false);
	let draft = $state('');
	let error = $state('');
	let input = $state<HTMLInputElement | null>(null);

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
		data.key;
		cancel();
	});
	$effect(() => {
		if (editing) input?.focus();
	});

	function title_keys(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			save();
		} else if (event.key === 'Escape') {
			// Before the shell's listener, which would close the pane.
			event.preventDefault();
			cancel();
		}
	}

	function copy_link() {
		if (!issue) return;
		writeText(`tracker://issues/${issue.key}`).then(
			() => push({ text: `Copied the link to ${issue.key}` }),
			() => push({ text: `Could not copy the link. Copy the key ${issue.key} instead.`, tone: 'error' })
		);
	}
</script>

{#if issue}
	<div class="flex h-12 shrink-0 items-center gap-1 border-b pr-2 pl-5">
		<span data-testid="detail-key" class="text-muted-foreground mr-1 font-mono text-xs">{issue.key}</span>
		<Button variant="ghost" size="icon-sm" class="text-muted-foreground size-7" aria-label="Copy link" title="Copy link" onclick={copy_link}>
			<LinkIcon class="size-4" strokeWidth={1.75} />
		</Button>
		<div class="ml-auto flex items-center gap-0.5">
			<Button
				data-testid="detail-prev"
				variant="ghost"
				size="icon-sm"
				class="text-muted-foreground size-7"
				aria-label="Previous issue (k)"
				title="Previous issue (k)"
				disabled={prev === null}
				onclick={() => prev && goto(`/issues/${prev}`)}
			>
				<ArrowUpIcon class="size-4" strokeWidth={1.75} />
			</Button>
			<Button
				data-testid="detail-next"
				variant="ghost"
				size="icon-sm"
				class="text-muted-foreground size-7"
				aria-label="Next issue (j)"
				title="Next issue (j)"
				disabled={next === null}
				onclick={() => next && goto(`/issues/${next}`)}
			>
				<ArrowDownIcon class="size-4" strokeWidth={1.75} />
			</Button>
			<Button
				data-testid="detail-close"
				variant="ghost"
				size="icon-sm"
				class="text-muted-foreground size-7"
				aria-label="Close (Esc)"
				title="Close (Esc)"
				href={list_href()}
			>
				<XIcon class="size-4" strokeWidth={1.75} />
			</Button>
		</div>
	</div>

	<div class="flex flex-col gap-6 p-5 pb-10">
		<div class="relative flex flex-col">
			{#if editing}
				<Input
					data-testid="title-input"
					class="font-display text-foreground -mx-2 h-9 w-[calc(100%+16px)] px-[7px] py-1 text-[22px] leading-tight font-medium tracking-[-0.02em] md:text-[22px]"
					aria-invalid={error ? 'true' : undefined}
					bind:ref={input}
					bind:value={draft}
					oninput={() => {
						if (error) error = check(draft);
					}}
					onkeydown={title_keys}
				/>
			{:else}
				<h2 data-testid="detail-title" class="font-display text-foreground text-[22px] leading-tight font-medium tracking-[-0.02em]">
					<button class="hover:bg-accent -mx-2 w-[calc(100%+16px)] rounded-md px-2 py-1 text-left" title="Edit the title" onclick={start}
						>{issue.title}</button
					>
				</h2>
			{/if}
			<!-- In the gap below, so clicking the title moves nothing. -->
			{#if editing}
				{#if error}
					<p data-testid="title-error" class="text-destructive absolute top-full mt-1 text-xs">{error}</p>
				{:else}
					<p class="text-muted-foreground absolute top-full mt-1 text-xs">Enter saves, Esc cancels.</p>
				{/if}
			{/if}
		</div>

		{#if description}
			<p data-testid="detail-description" class="text-ink-2 max-w-[62ch] text-[14px] leading-[1.6]">{description}</p>
		{/if}

		<Properties {issue} />

		<Thread key={issue.key} />
	</div>
{:else}
	<!-- Deleted while open; a key that never existed goes to issues/+error.svelte instead. -->
	<div class="p-5">
		<p data-testid="error-message" class="text-foreground text-base font-medium">No issue {data.key}.</p>
		<p class="text-muted-foreground mt-1 text-[13px]">
			It may have been deleted. <a class="text-primary" href="/issues">Back to issues</a>
		</p>
	</div>
{/if}
