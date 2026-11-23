<!--
	New issue (examples/tracker/src/lib/overlays/NewIssue.svelte), in gpuix-svelte-ui's Dialog
	(size lg: 640 px, centred; the Tracker's is 560 px at 14% from the top). The four Selects are
	gpuix-svelte-ui's: 180 px each at least, so they wrap to two lines, and show no glyphs.
	Enter in the title, or Cmd+Enter anywhere while the dialog is open, submits.
-->
<script lang="ts">
	import { focus_element, on_window_key, type ShadowNode } from 'gpuix-svelte';
	import { Checkbox, Dialog, Select, navigate, router } from 'gpuix-svelte-ui';
	import Btn from '../components/Btn.svelte';
	import LabelDot from '../components/LabelDot.svelte';
	import { LABELS, PEOPLE, PRIORITIES, PROJECTS, STATUSES, project_by_key, type PriorityId, type StatusId } from '../seed.ts';
	import { create, person } from '../store.svelte.ts';
	import { push, ui } from '../ui.svelte.ts';

	const MIN = 3;
	const MAX = 200;

	let title = $state('');
	let project = $state('ENG');
	let status = $state<StatusId>('backlog');
	let priority = $state<PriorityId>('none');
	let assignee = $state('none');
	let labels = $state<string[]>([]);
	let touched = $state(false);
	let input: ShadowNode | null = null;

	function problem(text: string): string {
		const length = text.trim().length;
		if (length === 0) return "Title can't be empty. Type at least 3 characters.";
		if (length < MIN) return `Type at least ${MIN} characters.`;
		if (length > MAX) return `Keep it to ${MAX} characters. This one has ${length}.`;
		return '';
	}
	const error = $derived(touched ? problem(title) : '');

	function reset() {
		const [, first, key] = router.path.split('/');
		title = '';
		project = first === 'projects' && project_by_key[key] ? key : 'ENG';
		status = 'backlog';
		priority = 'none';
		assignee = 'none';
		labels = [];
		touched = false;
	}

	// The Tracker resets the form when the dialog opens (onOpenAutoFocus).
	$effect.pre(() => {
		if (ui.new_issue) reset();
	});

	// Cmd/Ctrl+Enter anywhere in the dialog (the Tracker's keydown on DialogContent).
	$effect(() => {
		if (!ui.new_issue) return;
		return on_window_key('keydown', (e) => {
			if (e.key === 'enter' && (e.modifiers?.cmd || e.modifiers?.ctrl)) {
				e.preventDefault();
				submit();
			}
		});
	});

	function submit() {
		touched = true;
		if (problem(title)) {
			if (input) focus_element(input);
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
			action: { label: 'Open', run: () => navigate(`/issues/${issue.key}`) }
		});
	}

	function toggle_label(id: string, on: boolean) {
		labels = on ? [...labels, id] : labels.filter((l) => l !== id);
	}

	const project_options = PROJECTS.map((p) => ({ value: p.key, label: p.name }));
	const status_options = STATUSES.map((s) => ({ value: s.id, label: s.name }));
	const priority_options = PRIORITIES.map((p) => ({ value: p.id, label: p.name }));
	const assignee_options = $derived([{ value: 'none', label: 'Unassigned' }, ...PEOPLE.map((p) => ({ value: p.id, label: person(p.id)?.name ?? '' }))]);
</script>

<Dialog open={ui.new_issue} onclose={() => (ui.new_issue = false)} size="lg" title="New issue" testid="new-issue">
	<div class="form">
		<div class="field-group">
			<text class="label">Title</text>
			<input
				class="title"
				class:invalid={error !== ''}
				testid="new-issue-title"
				placeholder="What needs doing?"
				autofocus
				value={title}
				onchange={(e) => (title = e.value)}
				onsubmit={submit}
				{@attach (node: ShadowNode) => {
					input = node;
					return () => (input = null);
				}}
			/>
			<text class="help" class:error={error !== ''} testid="new-issue-error">{error || `${MIN} to ${MAX} characters.`}</text>
		</div>

		<div class="selects">
			<Select testid="new-issue-project" options={project_options} value={project} onchange={(v) => (project = v)} />
			<Select testid="new-issue-status" options={status_options} value={status} onchange={(v) => (status = v as StatusId)} />
			<Select testid="new-issue-priority" options={priority_options} value={priority} onchange={(v) => (priority = v as PriorityId)} />
			<Select testid="new-issue-assignee" options={assignee_options} value={assignee} onchange={(v) => (assignee = v)} />
		</div>

		<div class="field-group">
			<text class="label strong">Labels</text>
			<div class="labels">
				{#each LABELS as label (label.id)}
					<div class="choice">
						<Checkbox testid="new-issue-label" checked={labels.includes(label.id)} onchange={(on) => toggle_label(label.id, on)} />
						<LabelDot label={label.id} />
						<text>{label.name}</text>
					</div>
				{/each}
			</div>
		</div>
	</div>
	{#snippet actions()}
		<Btn variant="ghost" testid="new-issue-cancel" onclick={() => (ui.new_issue = false)}>Cancel</Btn>
		<Btn testid="new-issue-submit" onclick={submit}>
			<text>Create issue</text>
			<text class="hint">⌘ Enter</text>
		</Btn>
	{/snippet}
</Dialog>

<style>
	.form { display: flex; flex-direction: column; gap: 20px; }
	.field-group { display: flex; flex-direction: column; gap: 6px; }
	.label { font-size: 13px; color: var(--foreground); }
	.label.strong { font-weight: 500; }
	.title {
		height: 36px;
		padding: 0 10px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--input);
		font-size: 16px;
		color: var(--foreground);
	}
	.title.invalid { border-color: var(--destructive); }
	.help { min-height: 18px; font-size: 12px; color: var(--muted-foreground); }
	.help.error { color: var(--destructive); }
	.selects { display: flex; flex-direction: row; flex-wrap: wrap; gap: 8px; font-size: 13px; }
	.labels { display: grid; grid-template-columns: 3; column-gap: 16px; row-gap: 8px; }
	.choice { display: flex; flex-direction: row; align-items: center; gap: 8px; font-size: 13px; color: var(--ink-2); }
	.hint { font-family: JetBrains Mono; font-size: 11px; color: var(--primary-foreground-70); }
</style>
