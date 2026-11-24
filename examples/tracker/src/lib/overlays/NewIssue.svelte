<!--
	New issue (docs/tracker-design.md, New issue). Open state is `ui.new_issue`.
	The title is checked on submit, then live. Enter in the title or
	Cmd/Ctrl+Enter anywhere in the dialog submits. The issue goes to the top of
	the list, highlighted, with an Open toast.

	Placed without a translate: measurements ignore transforms, and Bits UI
	places each Select's list from its trigger's measured box.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Checkbox } from '#lib/components/ui/checkbox/index.ts';
	import { Dialog, DialogContent, DialogFooter, DialogTitle } from '#lib/components/ui/dialog/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { Label } from '#lib/components/ui/label/index.ts';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '#lib/components/ui/select/index.ts';
	import { Avatar, LabelDot, PriorityIcon, ProjectSquare, StatusIcon } from '#lib/glyphs/index.ts';
	import {
		LABELS,
		PEOPLE,
		PRIORITIES,
		PROJECTS,
		STATUSES,
		priority_by_id,
		project_by_key,
		status_by_id,
		type PriorityId,
		type StatusId
	} from '#lib/seed.ts';
	import { create, person } from '#lib/store.svelte.ts';
	import { push, ui } from '#lib/ui.svelte.ts';

	const MIN = 3;
	const MAX = 200;
	// Four in one row of the 560px dialog.
	const trigger = 'gap-1.5 px-2.5 text-[13px]';

	let title = $state('');
	let project = $state('ENG');
	// Backlog, the first group: a new issue lands on the list's first page, at the top.
	let status = $state<StatusId>('backlog');
	let priority = $state<PriorityId>('none');
	/** a person id, or 'none' */
	let assignee = $state('none');
	let labels = $state<string[]>([]);
	/** set by the first submit: from then on the title is checked as you type */
	let touched = $state(false);
	let input = $state<HTMLInputElement | null>(null);

	function problem(text: string): string {
		const length = text.trim().length;
		if (length === 0) return "Title can't be empty. Type at least 3 characters.";
		if (length < MIN) return `Type at least ${MIN} characters.`;
		if (length > MAX) return `Keep it to ${MAX} characters. This one has ${length}.`;
		return '';
	}
	const error = $derived(touched ? problem(title) : '');

	function reset() {
		const [, first, key] = page.url.pathname.split('/');
		title = '';
		project = first === 'projects' && project_by_key[key] ? key : 'ENG';
		status = 'backlog';
		priority = 'none';
		assignee = 'none';
		labels = [];
		touched = false;
	}

	function submit() {
		touched = true;
		if (problem(title)) {
			input?.focus();
			return;
		}
		const issue = create({
			title,
			project,
			status,
			priority,
			assignee: assignee === 'none' ? null : assignee,
			labelIds: LABELS.filter((l) => labels.includes(l.id)).map((l) => l.id)
		});
		ui.highlighted = issue.key;
		ui.new_issue = false;
		push({
			text: `Created ${issue.key}`,
			status: issue.status,
			action: { label: 'Open', run: () => goto(`/issues/${issue.key}`) }
		});
	}

	function toggle_label(id: string, on: boolean) {
		labels = on ? [...labels, id] : labels.filter((l) => l !== id);
	}

	const assignee_name = $derived(assignee === 'none' ? 'Unassigned' : (person(assignee)?.name ?? 'Unassigned'));
	const select_list = 'max-h-72 overflow-y-scroll';
</script>

<Dialog bind:open={ui.new_issue}>
	<DialogContent
		data-testid="new-issue"
		class="top-[14vh] right-0 left-0 mx-auto translate-x-0 translate-y-0 gap-5 rounded-lg p-5 shadow-sm sm:max-w-[560px]"
		onOpenAutoFocus={(event) => {
			event.preventDefault();
			reset();
			input?.focus();
		}}
		onkeydown={(event: KeyboardEvent) => {
			// Here, not in the shell's keymap: Bits UI's Checkbox prevents Enter's default, and the shell skips prevented keys.
			if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
				event.preventDefault();
				submit();
			}
		}}
	>
		<DialogTitle class="font-display text-foreground text-base font-medium tracking-[-0.01em]">New issue</DialogTitle>

		<div class="flex flex-col gap-1.5">
			<Label for="new-issue-title" class="text-foreground text-[13px]">Title</Label>
			<div class="relative">
				<Input
					id="new-issue-title"
					data-testid="new-issue-title"
					class="h-9 text-[16px] md:text-[16px]"
					aria-invalid={error ? 'true' : undefined}
					aria-describedby="new-issue-title-help"
					bind:ref={input}
					bind:value={title}
					onkeydown={(event: KeyboardEvent) => {
						if (event.key === 'Enter' && !event.metaKey && !event.ctrlKey) {
							event.preventDefault();
							submit();
						}
					}}
				/>
				<!-- Blitz draws no placeholder text. -->
				{#if !title}
					<span class="text-muted-foreground pointer-events-none absolute top-0 left-2.5 flex h-9 items-center text-[16px]">What needs doing?</span>
				{/if}
			</div>
			<p
				id="new-issue-title-help"
				data-testid="new-issue-error"
				class={['min-h-[18px] text-xs', error ? 'text-destructive' : 'text-muted-foreground']}
			>
				{error || `${MIN} to ${MAX} characters.`}
			</p>
		</div>

		<div class="flex flex-wrap gap-2">
			<Select type="single" bind:value={project}>
				<SelectTrigger data-testid="new-issue-project" size="sm" class={trigger} aria-label="Project">
					<ProjectSquare {project} />{project_by_key[project].name}
				</SelectTrigger>
				<SelectContent>
					{#each PROJECTS as p (p.key)}
						<SelectItem value={p.key} label={p.name}><ProjectSquare project={p.key} />{p.name}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
			<Select type="single" bind:value={status}>
				<SelectTrigger data-testid="new-issue-status" size="sm" class={trigger} aria-label="Status">
					<StatusIcon {status} />{status_by_id[status].name}
				</SelectTrigger>
				<SelectContent>
					{#each STATUSES as s (s.id)}
						<SelectItem value={s.id} label={s.name}><StatusIcon status={s.id} />{s.name}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
			<Select type="single" bind:value={priority}>
				<SelectTrigger data-testid="new-issue-priority" size="sm" class={trigger} aria-label="Priority">
					<PriorityIcon {priority} />{priority_by_id[priority].name}
				</SelectTrigger>
				<SelectContent>
					{#each PRIORITIES as p (p.id)}
						<SelectItem value={p.id} label={p.name}><PriorityIcon priority={p.id} />{p.name}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
			<Select type="single" bind:value={assignee}>
				<SelectTrigger data-testid="new-issue-assignee" size="sm" class={trigger} aria-label="Assignee">
					<Avatar id={assignee === 'none' ? null : assignee} />{assignee_name}
				</SelectTrigger>
				<SelectContent class={select_list}>
					<SelectItem value="none" label="Unassigned"><Avatar id={null} />Unassigned</SelectItem>
					{#each PEOPLE as p (p.id)}
						<SelectItem value={p.id} label={person(p.id)?.name}><Avatar id={p.id} />{person(p.id)?.name}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
		</div>

		<div class="flex flex-col gap-2">
			<span class="text-foreground text-[13px] font-medium">Labels</span>
			<div class="grid grid-cols-3 gap-x-4 gap-y-2">
				{#each LABELS as label (label.id)}
					<div class="flex items-center gap-2">
						<Checkbox
							id="new-issue-label-{label.id}"
							data-testid="new-issue-label"
							checked={labels.includes(label.id)}
							onCheckedChange={(on) => toggle_label(label.id, on)}
						/>
						<Label for="new-issue-label-{label.id}" class="text-ink-2 flex items-center gap-1.5 text-[13px] font-normal">
							<LabelDot label={label.id} />{label.name}
						</Label>
					</div>
				{/each}
			</div>
		</div>

		<DialogFooter class="gap-2">
			<Button variant="ghost" data-testid="new-issue-cancel" onclick={() => (ui.new_issue = false)}>Cancel</Button>
			<Button data-testid="new-issue-submit" onclick={submit}>
				Create issue
				<span class="text-primary-foreground/70 font-mono text-[11px]">⌘ Enter</span>
			</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

