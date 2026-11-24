<!--
	Comments and activity for one issue, and the box to comment in. The seed
	makes them lazily from the key, so they cost something only when shown.
	An <input>, not a <textarea>: typing into a textarea is untested in Blitz.
-->
<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { Tabs, TabsContent, TabsList, TabsTrigger } from '#lib/components/ui/tabs/index.ts';
	import { long_date, short_date } from '#lib/format.ts';
	import { Avatar, StatusIcon } from '#lib/glyphs/index.ts';
	import { add_comment, person, thread, undo_action } from '#lib/store.svelte.ts';
	import { push } from '#lib/ui.svelte.ts';

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

<Tabs bind:value={tab} class="gap-4">
	<TabsList variant="line" class="h-9 w-full justify-start gap-4 border-b p-0">
		<TabsTrigger data-testid="tab-comments" value="comments" class="flex-none px-0">
			Comments <span class="text-muted-foreground font-mono text-[11px]">{details.comments.length}</span>
		</TabsTrigger>
		<TabsTrigger data-testid="tab-activity" value="activity" class="flex-none px-0">
			Activity <span class="text-muted-foreground font-mono text-[11px]">{details.activity.length}</span>
		</TabsTrigger>
	</TabsList>
	<TabsContent value="comments" class="flex flex-col gap-4">
		{#each details.comments as comment, i (i)}
			<div data-testid="comment" class="flex gap-2.5">
				<Avatar id={comment.author} />
				<div class="flex min-w-0 flex-col gap-0.5">
					<div class="flex items-baseline gap-2">
						<span class="text-foreground text-[13px] font-medium">{person(comment.author)?.name}</span>
						<span class="text-muted-foreground font-mono text-[11px]">{long_date(comment.at)}</span>
					</div>
					<p class="text-ink-2 text-[13px]">{comment.body}</p>
				</div>
			</div>
		{:else}
			<p class="text-muted-foreground text-xs">No comments yet.</p>
		{/each}
	</TabsContent>
	<TabsContent value="activity" class="flex flex-col gap-2">
		{#each details.activity as entry, i (i)}
			<div data-testid="activity" class="text-muted-foreground flex min-w-0 items-center gap-2 text-xs">
				{#if entry.status}
					<StatusIcon status={entry.status} size={12} />
				{:else}
					<span class="bg-muted-foreground/50 mx-[3px] size-1.5 shrink-0 rounded-full"></span>
				{/if}
				<span class="truncate"
					><span class="text-ink-2 font-medium">{person(entry.author)?.name}</span> {entry.text} · {short_date(entry.at)}</span
				>
			</div>
		{/each}
	</TabsContent>
</Tabs>

<div class="flex items-center gap-2 pt-2">
	<div class="relative flex min-w-0 flex-1">
		<Input
			data-testid="comment-input"
			class="h-8 text-[14px] md:text-[14px]"
			placeholder="Leave a comment…"
			bind:value={draft}
			onkeydown={(event) => {
				if (event.key === 'Enter') {
					event.preventDefault();
					post();
				}
				// Escape leaves the field and keeps the draft, rather than closing the pane.
				if (event.key === 'Escape') {
					event.preventDefault();
					(event.currentTarget as HTMLElement).blur();
				}
			}}
		/>
		{#if !draft}
			<!-- Blitz draws no placeholder text. -->
			<span class="text-muted-foreground pointer-events-none absolute top-[7px] left-2.5 text-[14px] leading-[18px]">Leave a comment…</span>
		{/if}
	</div>
	<Button data-testid="comment-submit" size="sm" variant="outline" disabled={!draft.trim()} onclick={post}>Comment</Button>
</div>
